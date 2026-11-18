//! Evaluator-only Monte Carlo estimates of selective decision risk.
//!
//! This module keeps generating-model identity inside the evaluator. The
//! candidate-facing policy receives only sampled observations, never the
//! truth label used to stratify the risk estimate. Wilson intervals are
//! per-model fixed-sample intervals; they are not sequentially valid bounds.

use crate::experiment_design::{evaluate_input_profile, ExperimentDesignScenario};
use crate::noise_robustness::SplitMix64;
use crate::sequential_evidence::{
    decide_sequential_stop, validate_action_catalogue, ActionObservationV1,
    ModelPriorV1, SequentialActionV1, SequentialDecisionV1,
    SequentialEvidenceError, SequentialPolicyConfigV1,
};
use std::fmt;

const WILSON_Z_95: f64 = 1.959_963_984_540_054;
const MAX_RISK_REPLICATES_PER_MODEL: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WilsonRateV1 {
    pub successes: usize,
    pub trials: usize,
    pub estimate: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelSelectiveRiskV1 {
    pub model_id: u32,
    pub family_id: u32,
    pub prior_probability: f64,
    pub replicates: usize,
    pub wrong_model_commit: WilsonRateV1,
    pub wrong_family_commit: WilsonRateV1,
    pub in_family_out_of_model_rejection: WilsonRateV1,
    pub family_parameter_not_identified: WilsonRateV1,
    pub other_abstention: WilsonRateV1,
    pub correct_model_commit: WilsonRateV1,
    pub mean_brier_score: f64,
    pub mean_log_loss: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionSelectiveRiskEstimateV1 {
    pub action_id: u32,
    pub seed: u64,
    pub replicates_per_model: usize,
    pub expected_wrong_model_commit_probability: f64,
    pub expected_wrong_family_commit_probability: f64,
    pub expected_in_family_ood_rejection_probability: f64,
    pub expected_family_parameter_unidentified_probability: f64,
    pub expected_other_abstention_probability: f64,
    pub expected_correct_model_commit_probability: f64,
    pub expected_brier_score: f64,
    pub expected_log_loss: f64,
    pub wrong_family_commit_standard_error: f64,
    pub per_model: Vec<ModelSelectiveRiskV1>,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeldOutModelSelectiveRiskV1 {
    pub action_id: u32,
    pub held_out_model_id: u32,
    pub seed: u64,
    pub replicates: usize,
    /// Commit to any registered model or family when truth is unregistered.
    pub false_candidate_acceptance: WilsonRateV1,
    pub correct_out_of_model_abstention: WilsonRateV1,
    pub other_abstention: WilsonRateV1,
    pub mean_max_candidate_posterior: f64,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskAdjustedActionScoreV1 {
    pub action_id: u32,
    pub eig_per_cost_nats: f64,
    pub false_family_commit_probability: f64,
    /// User-declared penalty measured in nats per unit false-commit probability.
    pub false_commit_penalty_nats: f64,
    pub risk_adjusted_utility_per_cost: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectiveRiskError {
    Sequential(SequentialEvidenceError),
    UnknownAction(u32),
    ActionAlreadyObserved,
    InvalidReplicateCount,
    ExcessiveReplicateCount,
    InvalidPolicyThreshold,
    InvalidEvaluatorScenario,
    HeldOutModelIsRegistered(u32),
    HeldOutModelMissing(u32),
    MissingPrediction(u32),
    InvalidPredictionShape,
    NonFiniteSample,
}

impl fmt::Display for SelectiveRiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sequential(error) => write!(f, "selective risk evaluation rejected: {error}"),
            Self::UnknownAction(id) => write!(f, "risk evaluation references unknown action {id}"),
            Self::ActionAlreadyObserved => {
                write!(f, "risk action already occurs in campaign history")
            }
            Self::InvalidReplicateCount => {
                write!(f, "risk estimator requires at least one replicate")
            }
            Self::ExcessiveReplicateCount => {
                write!(f, "risk replicate count exceeds the bounded reference limit")
            }
            Self::InvalidPolicyThreshold => write!(f, "risk policy threshold is invalid"),
            Self::InvalidEvaluatorScenario => {
                write!(f, "evaluator scenario does not preserve the candidate experiment")
            }
            Self::HeldOutModelIsRegistered(id) => {
                write!(f, "held-out model {id} is already in the candidate set")
            }
            Self::HeldOutModelMissing(id) => write!(f, "held-out evaluator model {id} is missing"),
            Self::MissingPrediction(id) => {
                write!(f, "evaluator prediction for model {id} is missing")
            }
            Self::InvalidPredictionShape => {
                write!(f, "evaluator prediction does not match the action output shape")
            }
            Self::NonFiniteSample => write!(f, "evaluator generated a non-finite observation"),
        }
    }
}

impl std::error::Error for SelectiveRiskError {}

/// Estimate one-step selective risk conditional on an existing history.
/// Each registered model is sampled in its own stratum. This is evaluator-only:
/// the true model ID is used to generate the trace and score the policy result,
/// but is never passed into the policy update or its observation structure.
pub fn estimate_action_selective_risk_evaluator_only(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    action_id: u32,
    config: SequentialPolicyConfigV1,
    replicates_per_model: usize,
    seed: u64,
) -> Result<ActionSelectiveRiskEstimateV1, SelectiveRiskError> {
    validate_action_catalogue(scenario, actions, prior)
        .map_err(SelectiveRiskError::Sequential)?;
    validate_risk_config(config)?;
    if replicates_per_model == 0 {
        return Err(SelectiveRiskError::InvalidReplicateCount);
    }
    if replicates_per_model > MAX_RISK_REPLICATES_PER_MODEL {
        return Err(SelectiveRiskError::ExcessiveReplicateCount);
    }
    let action = actions
        .iter()
        .find(|action| action.action_id() == action_id)
        .ok_or(SelectiveRiskError::UnknownAction(action_id))?;
    if history.iter().any(|entry| entry.action_id == action_id) {
        return Err(SelectiveRiskError::ActionAlreadyObserved);
    }
    let spent_cost = campaign_cost(actions, history)?;
    if spent_cost + action.cost() > config.maximum_total_cost + 1.0e-10 {
        return Err(SelectiveRiskError::Sequential(
            SequentialEvidenceError::InvalidCampaignCost,
        ));
    }

    let mut per_model = Vec::with_capacity(scenario.models.len());
    for prior_entry in prior {
        let model = scenario
            .models
            .iter()
            .find(|model| model.id == prior_entry.model_id)
            .ok_or(SelectiveRiskError::MissingPrediction(prior_entry.model_id))?;
        let means = action
            .model_means_for(model.id)
            .ok_or(SelectiveRiskError::MissingPrediction(model.id))?;
        let flattened_means: Vec<f64> =
            means.observations.iter().flatten().copied().collect();
        let mut rng = SplitMix64::new(seed_for(seed, action_id, model.id));
        let mut counts = ModelCounts::default();
        let mut brier_sum = 0.0;
        let mut log_loss_sum = 0.0;

        for _replicate in 0..replicates_per_model {
            let noise = action
                .covariance()
                .covariance()
                .sample_gaussian(&mut rng)
                .map_err(|error| {
                    SelectiveRiskError::Sequential(
                        SequentialEvidenceError::Covariance(error),
                    )
                })?;
            let values = add_noise(&flattened_means, &noise)?;
            let observation = ActionObservationV1 {
                action_id,
                sensor_ids: action.selected_sensor_ids().to_vec(),
                times: scenario.observation_times.clone(),
                values: unflatten(
                    &values,
                    scenario.observation_times.len(),
                    action.selected_sensor_ids().len(),
                )?,
            };
            let mut next_history = history.to_vec();
            next_history.push(observation);
            let mut terminal_config = config;
            terminal_config.maximum_experiments = next_history.len();
            terminal_config.maximum_total_cost = spent_cost + action.cost();
            let report = decide_sequential_stop(
                scenario,
                actions,
                prior,
                &next_history,
                terminal_config,
            )
            .map_err(SelectiveRiskError::Sequential)?;
            classify_decision(
                &mut counts,
                &report.decision,
                model.id,
                prior_entry.family_id,
            );

            let mut squared_error = 0.0;
            let mut true_probability = 1.0e-15;
            for weight in &report.posterior.weights {
                let target = if weight.model_id == model.id {
                    true_probability = weight.probability.max(1.0e-15);
                    1.0
                } else {
                    0.0
                };
                squared_error += (weight.probability - target).powi(2);
            }
            brier_sum += squared_error;
            log_loss_sum += -true_probability.ln();
        }

        per_model.push(ModelSelectiveRiskV1 {
            model_id: model.id,
            family_id: prior_entry.family_id,
            prior_probability: prior_entry.probability,
            replicates: replicates_per_model,
            wrong_model_commit: wilson(counts.wrong_model_commit, replicates_per_model),
            wrong_family_commit: wilson(counts.wrong_family_commit, replicates_per_model),
            in_family_out_of_model_rejection: wilson(
                counts.in_family_out_of_model_rejection,
                replicates_per_model,
            ),
            family_parameter_not_identified: wilson(
                counts.family_parameter_not_identified,
                replicates_per_model,
            ),
            other_abstention: wilson(counts.other_abstention, replicates_per_model),
            correct_model_commit: wilson(counts.correct_model_commit, replicates_per_model),
            mean_brier_score: brier_sum / replicates_per_model as f64,
            mean_log_loss: log_loss_sum / replicates_per_model as f64,
        });
    }

    let weighted = |select: fn(&ModelSelectiveRiskV1) -> f64| {
        per_model
            .iter()
            .map(|entry| entry.prior_probability * select(entry))
            .sum::<f64>()
    };
    let wrong_family_commit_standard_error = per_model
        .iter()
        .map(|entry| {
            let p = entry.prior_probability;
            let rate = entry.wrong_family_commit.estimate;
            p * p * rate * (1.0 - rate) / entry.replicates as f64
        })
        .sum::<f64>()
        .sqrt();

    Ok(ActionSelectiveRiskEstimateV1 {
        action_id,
        seed,
        replicates_per_model,
        expected_wrong_model_commit_probability: weighted(|entry| {
            entry.wrong_model_commit.estimate
        }),
        expected_wrong_family_commit_probability: weighted(|entry| {
            entry.wrong_family_commit.estimate
        }),
        expected_in_family_ood_rejection_probability: weighted(|entry| {
            entry.in_family_out_of_model_rejection.estimate
        }),
        expected_family_parameter_unidentified_probability: weighted(|entry| {
            entry.family_parameter_not_identified.estimate
        }),
        expected_other_abstention_probability: weighted(|entry| {
            entry.other_abstention.estimate
        }),
        expected_correct_model_commit_probability: weighted(|entry| {
            entry.correct_model_commit.estimate
        }),
        expected_brier_score: weighted(|entry| entry.mean_brier_score),
        expected_log_loss: weighted(|entry| entry.mean_log_loss),
        wrong_family_commit_standard_error,
        per_model,
        assumptions: vec![
            "Monte Carlo truth is stratified by registered model and weighted by the prior",
            "action observations are conditionally independent given the candidate model",
            "policy thresholds are fixed inputs and are not sequentially calibrated",
            "Wilson intervals are fixed-sample per-model intervals",
            "risk is conditional on the candidate set and evaluator distribution",
        ],
    })
}

/// Estimate false acceptance of a registered model family when the
/// evaluator's generating graph is held out of the candidate set.
///
/// The evaluator scenario must contain the exact candidate models plus one
/// held-out model, with the same state basis, sensors, schedule and budgets.
/// Only the candidate scenario, action and sampled observation are supplied to
/// the sequential policy. The held-out model ID remains evaluator-local.
pub fn estimate_held_out_model_false_acceptance_evaluator_only(
    candidate_scenario: &ExperimentDesignScenario,
    evaluator_scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    action_id: u32,
    held_out_model_id: u32,
    config: SequentialPolicyConfigV1,
    replicates: usize,
    seed: u64,
) -> Result<HeldOutModelSelectiveRiskV1, SelectiveRiskError> {
    validate_action_catalogue(candidate_scenario, actions, prior)
        .map_err(SelectiveRiskError::Sequential)?;
    validate_risk_config(config)?;
    if replicates == 0 {
        return Err(SelectiveRiskError::InvalidReplicateCount);
    }
    if replicates > MAX_RISK_REPLICATES_PER_MODEL {
        return Err(SelectiveRiskError::ExcessiveReplicateCount);
    }
    if candidate_scenario
        .models
        .iter()
        .any(|model| model.id == held_out_model_id)
    {
        return Err(SelectiveRiskError::HeldOutModelIsRegistered(held_out_model_id));
    }
    evaluator_scenario
        .validate()
        .map_err(|_| SelectiveRiskError::InvalidEvaluatorScenario)?;
    if evaluator_scenario.models.len() != candidate_scenario.models.len() + 1
        || evaluator_scenario.sensors != candidate_scenario.sensors
        || evaluator_scenario.noise_covariance != candidate_scenario.noise_covariance
        || evaluator_scenario.observation_times != candidate_scenario.observation_times
        || evaluator_scenario.sensor_budget != candidate_scenario.sensor_budget
        || evaluator_scenario.max_abs_forcing != candidate_scenario.max_abs_forcing
        || evaluator_scenario.input_energy_budget != candidate_scenario.input_energy_budget
    {
        return Err(SelectiveRiskError::InvalidEvaluatorScenario);
    }
    for candidate in &candidate_scenario.models {
        let evaluator_copy = evaluator_scenario
            .models
            .iter()
            .find(|model| model.id == candidate.id)
            .ok_or(SelectiveRiskError::InvalidEvaluatorScenario)?;
        if evaluator_copy != candidate {
            return Err(SelectiveRiskError::InvalidEvaluatorScenario);
        }
    }
    let held_out = evaluator_scenario
        .models
        .iter()
        .find(|model| model.id == held_out_model_id)
        .ok_or(SelectiveRiskError::HeldOutModelMissing(held_out_model_id))?;

    let action = actions
        .iter()
        .find(|action| action.action_id() == action_id)
        .ok_or(SelectiveRiskError::UnknownAction(action_id))?;
    if history.iter().any(|entry| entry.action_id == action_id) {
        return Err(SelectiveRiskError::ActionAlreadyObserved);
    }
    let spent_cost = campaign_cost(actions, history)?;
    if spent_cost + action.cost() > config.maximum_total_cost + 1.0e-10 {
        return Err(SelectiveRiskError::Sequential(
            SequentialEvidenceError::InvalidCampaignCost,
        ));
    }

    // Generate predictions only inside the evaluator scenario. The candidate
    // scenario and all policy calls below remain free of the held-out model.
    let evaluator_score = evaluate_input_profile(
        evaluator_scenario,
        action.profile(),
        action.selected_sensor_ids(),
    )
    .map_err(|_| SelectiveRiskError::InvalidEvaluatorScenario)?;
    let held_out_means = evaluator_score
        .model_observation_means
        .iter()
        .find(|means| means.model_id == held_out.id)
        .ok_or(SelectiveRiskError::MissingPrediction(held_out_model_id))?;
    if held_out_means.observations.len() != candidate_scenario.observation_times.len()
        || held_out_means
            .observations
            .iter()
            .any(|row| row.len() != action.selected_sensor_ids().len())
    {
        return Err(SelectiveRiskError::InvalidPredictionShape);
    }
    let flattened_means: Vec<f64> =
        held_out_means.observations.iter().flatten().copied().collect();
    let mut rng = SplitMix64::new(seed_for(seed, action_id, held_out_model_id));
    let mut false_accepts = 0_usize;
    let mut correct_ood_abstentions = 0_usize;
    let mut other_abstentions = 0_usize;
    let mut maximum_posterior_sum = 0.0;

    for _ in 0..replicates {
        let noise = action
            .covariance()
            .covariance()
            .sample_gaussian(&mut rng)
            .map_err(|error| {
                SelectiveRiskError::Sequential(
                    SequentialEvidenceError::Covariance(error),
                )
            })?;
        let values = add_noise(&flattened_means, &noise)?;
        let observation = ActionObservationV1 {
            action_id,
            sensor_ids: action.selected_sensor_ids().to_vec(),
            times: candidate_scenario.observation_times.clone(),
            values: unflatten(
                &values,
                candidate_scenario.observation_times.len(),
                action.selected_sensor_ids().len(),
            )?,
        };
        let mut next_history = history.to_vec();
        next_history.push(observation);
        let mut terminal_config = config;
        terminal_config.maximum_experiments = next_history.len();
        terminal_config.maximum_total_cost = spent_cost + action.cost();
        let report = decide_sequential_stop(
            candidate_scenario,
            actions,
            prior,
            &next_history,
            terminal_config,
        )
        .map_err(SelectiveRiskError::Sequential)?;
        maximum_posterior_sum += report
            .posterior
            .weights
            .iter()
            .map(|weight| weight.probability)
            .fold(0.0_f64, f64::max);
        match report.decision {
            SequentialDecisionV1::CommitModel { .. }
            | SequentialDecisionV1::CommitFamilyParameterNotIdentified { .. } => {
                false_accepts += 1;
            }
            SequentialDecisionV1::AbstainOutOfModelSet => {
                correct_ood_abstentions += 1;
            }
            _ => {
                other_abstentions += 1;
            }
        }
    }

    Ok(HeldOutModelSelectiveRiskV1 {
        action_id,
        held_out_model_id,
        seed,
        replicates,
        false_candidate_acceptance: wilson(false_accepts, replicates),
        correct_out_of_model_abstention: wilson(correct_ood_abstentions, replicates),
        other_abstention: wilson(other_abstentions, replicates),
        mean_max_candidate_posterior: maximum_posterior_sum / replicates as f64,
        assumptions: vec![
            "held-out model is evaluator-only and absent from candidate prior and policy",
            "candidate models are copied unchanged into evaluator scenario",
            "held-out and in-family observations use the action's identity-bound covariance",
            "action noises are conditionally independent of earlier campaign actions",
            "false candidate acceptance includes model and family commits",
            "Monte Carlo/Wilson rates are fixed-sample, not sequentially valid",
        ],
    })
}

/// Apply a transparent penalty to EIG per cost. This is a user-specified
/// decision utility, not the exact evidential-bias criterion from the paper.
pub fn risk_adjusted_action_score(
    eig: &crate::sequential_evidence::ExpectedInformationGainV1,
    risk: &ActionSelectiveRiskEstimateV1,
    false_commit_penalty_nats: f64,
) -> Result<RiskAdjustedActionScoreV1, SelectiveRiskError> {
    if eig.action_id != risk.action_id
        || !false_commit_penalty_nats.is_finite()
        || false_commit_penalty_nats < 0.0
    {
        return Err(SelectiveRiskError::InvalidPolicyThreshold);
    }
    let utility = eig.gain_per_cost
        - false_commit_penalty_nats * risk.expected_wrong_family_commit_probability;
    Ok(RiskAdjustedActionScoreV1 {
        action_id: eig.action_id,
        eig_per_cost_nats: eig.gain_per_cost,
        false_family_commit_probability: risk.expected_wrong_family_commit_probability,
        false_commit_penalty_nats,
        risk_adjusted_utility_per_cost: utility,
    })
}

#[derive(Default)]
struct ModelCounts {
    wrong_model_commit: usize,
    wrong_family_commit: usize,
    in_family_out_of_model_rejection: usize,
    family_parameter_not_identified: usize,
    other_abstention: usize,
    correct_model_commit: usize,
}

fn classify_decision(
    counts: &mut ModelCounts,
    decision: &SequentialDecisionV1,
    true_model_id: u32,
    true_family_id: u32,
) {
    match decision {
        SequentialDecisionV1::CommitModel {
            model_id,
            family_id,
            ..
        } => {
            if *model_id == true_model_id {
                counts.correct_model_commit += 1;
            } else {
                counts.wrong_model_commit += 1;
            }
            if *family_id != true_family_id {
                counts.wrong_family_commit += 1;
            }
        }
        SequentialDecisionV1::CommitFamilyParameterNotIdentified {
            family_id, ..
        } => {
            counts.family_parameter_not_identified += 1;
            if *family_id != true_family_id {
                counts.wrong_family_commit += 1;
            }
        }
        SequentialDecisionV1::AbstainOutOfModelSet => {
            counts.in_family_out_of_model_rejection += 1;
        }
        _ => counts.other_abstention += 1,
    }
}

fn wilson(successes: usize, trials: usize) -> WilsonRateV1 {
    let n = trials as f64;
    let estimate = successes as f64 / n;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let center = (estimate + z2 / (2.0 * n)) / denominator;
    let half_width = WILSON_Z_95
        * ((estimate * (1.0 - estimate) / n + z2 / (4.0 * n * n)).sqrt())
        / denominator;
    WilsonRateV1 {
        successes,
        trials,
        estimate,
        lower_95: (center - half_width).max(0.0),
        upper_95: (center + half_width).min(1.0),
    }
}

fn validate_risk_config(config: SequentialPolicyConfigV1) -> Result<(), SelectiveRiskError> {
    if !config.posterior_commit_threshold.is_finite()
        || config.posterior_commit_threshold <= 0.0
        || config.posterior_commit_threshold >= 1.0
        || !config.maximum_campaign_residual.is_finite()
        || config.maximum_campaign_residual < 0.0
        || config.maximum_experiments == 0
        || config.eig_replicates_per_model == 0
        || !config.maximum_total_cost.is_finite()
        || config.maximum_total_cost <= 0.0
    {
        return Err(SelectiveRiskError::InvalidPolicyThreshold);
    }
    Ok(())
}

fn campaign_cost(
    actions: &[SequentialActionV1],
    history: &[ActionObservationV1],
) -> Result<f64, SelectiveRiskError> {
    history
        .iter()
        .map(|observation| {
            actions
                .iter()
                .find(|action| action.action_id() == observation.action_id)
                .map(|action| action.cost())
                .ok_or(SelectiveRiskError::UnknownAction(observation.action_id))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|costs| costs.iter().sum())
}

fn add_noise(means: &[f64], noise: &[f64]) -> Result<Vec<f64>, SelectiveRiskError> {
    if means.len() != noise.len() {
        return Err(SelectiveRiskError::InvalidPredictionShape);
    }
    let values: Vec<f64> = means
        .iter()
        .zip(noise.iter())
        .map(|(mean, error)| mean + error)
        .collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SelectiveRiskError::NonFiniteSample);
    }
    Ok(values)
}

fn unflatten(
    values: &[f64],
    time_count: usize,
    sensor_count: usize,
) -> Result<Vec<Vec<f64>>, SelectiveRiskError> {
    if sensor_count == 0 || time_count.checked_mul(sensor_count) != Some(values.len()) {
        return Err(SelectiveRiskError::InvalidPredictionShape);
    }
    Ok(values.chunks(sensor_count).map(|row| row.to_vec()).collect())
}

fn seed_for(seed: u64, action_id: u32, model_id: u32) -> u64 {
    seed ^ (u64::from(action_id) << 32)
        ^ u64::from(model_id).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{
        ExperimentDesignScenario, ForcingSegment, InputProfile, ModelCandidate,
    };
    use crate::network::{Edge, NetworkScenario};
    use crate::noise_robustness::{
        Ar1KroneckerCovariance, DenseSpdCovariance, ObservationCovarianceV1,
    };
    use crate::sensor_design::SensorCandidate;
    use crate::sequential_evidence::{
        prepare_sequential_action, ModelPriorV1, SequentialPolicyConfigV1,
    };
    use crate::total_kl_trust_region::ObservationDesignIdentityV1;

    fn scenario() -> ExperimentDesignScenario {
        let make_network = |bypass: bool| {
            let mut edges = vec![
                Edge { left: 0, right: 1, conductance: 1.0 },
                Edge { left: 1, right: 2, conductance: 1.0 },
            ];
            if bypass {
                edges.push(Edge { left: 0, right: 2, conductance: 0.4 });
            }
            NetworkScenario {
                name: if bypass { "risk-bypass" } else { "risk-chain" },
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![0.2, 0.0, 0.0],
                edges,
                horizon: 2.0,
            }
        };
        ExperimentDesignScenario {
            name: "risk-chain-bypass",
            models: vec![
                ModelCandidate { id: 1, label: "chain", network: make_network(false) },
                ModelCandidate { id: 2, label: "bypass", network: make_network(true) },
            ],
            sensors: vec![
                SensorCandidate { id: 10, node: 1, cost: 1.0, common_cause_group: None },
                SensorCandidate { id: 11, node: 2, cost: 1.0, common_cause_group: None },
            ],
            noise_covariance: vec![vec![0.02, 0.005], vec![0.005, 0.03]],
            observation_times: vec![0.4, 0.8, 1.2, 1.6, 2.0],
            sensor_budget: 2.0,
            max_abs_forcing: 2.0,
            input_energy_budget: 0.5,
        }
    }

    fn action(fixture: &ExperimentDesignScenario) -> SequentialActionV1 {
        let profile = InputProfile {
            id: 1,
            label: "risk-source-pulse",
            segments: vec![
                ForcingSegment {
                    start_time: 0.0,
                    end_time: 0.5,
                    forcing: vec![1.0, 0.0, 0.0],
                },
                ForcingSegment {
                    start_time: 0.5,
                    end_time: 2.0,
                    forcing: vec![0.0, 0.0, 0.0],
                },
            ],
        };
        let covariance_id = "covariance:risk-ar1".to_owned();
        let process_id = "process:risk-ar1".to_owned();
        let sampling_id = "sampling:risk-uniform-five".to_owned();
        let operator_id = "operator:risk-sensors".to_owned();
        let cutoff_id = "availability:risk-all-observations".to_owned();
        let covariance = ObservationCovarianceV1::new(
            covariance_id.clone(),
            process_id.clone(),
            sampling_id.clone(),
            operator_id.clone(),
            cutoff_id.clone(),
            vec![10, 11],
            fixture.observation_times.clone(),
            Ar1KroneckerCovariance::new(
                fixture.observation_times.len(),
                0.6,
                DenseSpdCovariance::new(fixture.noise_covariance.clone()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let identity = ObservationDesignIdentityV1 {
            covariance_id,
            observation_process_id: process_id,
            sampling_policy_id: sampling_id,
            observation_operator_id: operator_id,
            availability_cutoff_id: cutoff_id,
        };
        prepare_sequential_action(
            fixture,
            1,
            profile,
            vec![10, 11],
            covariance,
            identity,
            1.0,
        )
        .unwrap()
    }

    fn prior() -> Vec<ModelPriorV1> {
        vec![
            ModelPriorV1 { model_id: 1, family_id: 100, probability: 0.5 },
            ModelPriorV1 { model_id: 2, family_id: 200, probability: 0.5 },
        ]
    }

    fn config() -> SequentialPolicyConfigV1 {
        SequentialPolicyConfigV1 {
            posterior_commit_threshold: 0.95,
            maximum_campaign_residual: 16.0,
            maximum_experiments: 1,
            maximum_total_cost: 1.0,
            eig_replicates_per_model: 32,
            eig_seed: 9,
        }
    }

    #[test]
    fn evaluator_risk_estimate_is_seeded_and_reports_wilson_intervals() {
        let fixture = scenario();
        let action = action(&fixture);
        let first = estimate_action_selective_risk_evaluator_only(
            &fixture, &[action.clone()], &prior(), &[], 1, config(), 64, 51,
        )
        .unwrap();
        let second = estimate_action_selective_risk_evaluator_only(
            &fixture, &[action], &prior(), &[], 1, config(), 64, 51,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.per_model.len(), 2);
        for model in &first.per_model {
            assert!(model.wrong_family_commit.lower_95 >= 0.0);
            assert!(model.wrong_family_commit.upper_95 <= 1.0);
            assert!(model.mean_brier_score.is_finite());
            assert!(model.mean_log_loss.is_finite());
        }
    }

    #[test]
    fn held_out_duplicate_model_can_be_false_accepted_as_registered_family() {
        let candidate = scenario();
        let action = action(&candidate);
        let mut evaluator = candidate.clone();
        evaluator.models.push(crate::experiment_design::ModelCandidate {
            id: 99,
            label: "held-out-duplicate-of-chain",
            network: candidate.models[0].network.clone(),
        });
        let same_family_prior = vec![
            ModelPriorV1 { model_id: 1, family_id: 100, probability: 0.5 },
            ModelPriorV1 { model_id: 2, family_id: 100, probability: 0.5 },
        ];
        let mut policy = config();
        policy.posterior_commit_threshold = 0.5;
        policy.maximum_campaign_residual = 1.0e6;
        let estimate = estimate_held_out_model_false_acceptance_evaluator_only(
            &candidate,
            &evaluator,
            &[action],
            &same_family_prior,
            &[],
            1,
            99,
            policy,
            64,
            123,
        )
        .unwrap();
        assert_eq!(estimate.false_candidate_acceptance.trials, 64);
        assert!(estimate.false_candidate_acceptance.estimate > 0.0);
        assert!(estimate.mean_max_candidate_posterior.is_finite());
    }

    #[test]
    fn held_out_model_is_rejected_when_candidate_set_leaks_its_id() {
        let candidate = scenario();
        let action = action(&candidate);
        let mut evaluator = candidate.clone();
        evaluator.models.push(crate::experiment_design::ModelCandidate {
            id: 99,
            label: "held-out-variant",
            network: candidate.models[0].network.clone(),
        });
        let mut leaking_candidate = candidate.clone();
        leaking_candidate.models.push(evaluator.models[2].clone());
        assert_eq!(
            estimate_held_out_model_false_acceptance_evaluator_only(
                &leaking_candidate,
                &evaluator,
                &[action],
                &prior(),
                &[],
                1,
                99,
                config(),
                8,
                9,
            ),
            Err(SelectiveRiskError::HeldOutModelIsRegistered(99))
        );
    }

    #[test]
    fn wilson_interval_contains_empirical_rate_and_is_bounded() {
        let interval = wilson(5, 20);
        assert!((interval.estimate - 0.25).abs() < 1.0e-12);
        assert!(interval.lower_95 <= interval.estimate);
        assert!(interval.upper_95 >= interval.estimate);
        assert!(interval.lower_95 >= 0.0);
        assert!(interval.upper_95 <= 1.0);
    }

    #[test]
    fn risk_adjusted_score_penalizes_false_confident_family_commits() {
        let eig = crate::sequential_evidence::ExpectedInformationGainV1 {
            action_id: 1,
            expected_information_gain_nats: 0.8,
            standard_error_nats: 0.02,
            replicates_per_model: 128,
            seed: 4,
            gain_per_cost: 0.8,
        };
        let risk = ActionSelectiveRiskEstimateV1 {
            action_id: 1,
            seed: 4,
            replicates_per_model: 128,
            expected_wrong_model_commit_probability: 0.1,
            expected_wrong_family_commit_probability: 0.2,
            expected_in_family_ood_rejection_probability: 0.05,
            expected_family_parameter_unidentified_probability: 0.1,
            expected_other_abstention_probability: 0.1,
            expected_correct_model_commit_probability: 0.45,
            expected_brier_score: 0.2,
            expected_log_loss: 0.4,
            wrong_family_commit_standard_error: 0.03,
            per_model: Vec::new(),
            assumptions: Vec::new(),
        };
        let score = risk_adjusted_action_score(&eig, &risk, 1.0).unwrap();
        assert!((score.risk_adjusted_utility_per_cost - 0.6).abs() < 1.0e-12);
    }
}
