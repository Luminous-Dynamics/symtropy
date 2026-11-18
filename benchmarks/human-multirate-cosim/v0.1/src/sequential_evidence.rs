//! Sequential Bayesian experiment selection with explicit model-set adequacy.
//!
//! Candidate policy inputs contain only declared model predictions, action
//! definitions and observed traces. Hidden generating truth is absent from all
//! policy-facing structures. Expected information gain is a seeded Monte Carlo
//! estimate, not an exact integral or a sequential false-commit guarantee.

use crate::experiment_design::{
    evaluate_input_profile, ExperimentDesignScenario, ExcitationDesignError,
    InputProfile, ModelObservationMeans,
};
use crate::noise_robustness::{CovarianceError, ObservationCovarianceV1, SplitMix64};
use crate::total_kl_trust_region::ObservationDesignIdentityV1;
use std::collections::{HashMap, HashSet};
use std::fmt;

const TOLERANCE: f64 = 1.0e-10;
const MAX_ACTIONS: usize = 32;
const MAX_EIG_REPLICATES_PER_MODEL: usize = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPriorV1 {
    pub model_id: u32,
    pub family_id: u32,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialActionV1 {
    pub action_id: u32,
    pub profile: InputProfile,
    pub selected_sensor_ids: Vec<u32>,
    pub covariance: ObservationCovarianceV1,
    pub expected_identity: ObservationDesignIdentityV1,
    pub cost: f64,
    pub model_means: Vec<ModelObservationMeans>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionObservationV1 {
    pub action_id: u32,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosteriorWeightV1 {
    pub model_id: u32,
    pub family_id: u32,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosteriorReportV1 {
    pub weights: Vec<PosteriorWeightV1>,
    pub model_entropy_nats: f64,
    pub family_probabilities: Vec<(u32, f64)>,
    pub family_entropy_nats: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedInformationGainV1 {
    pub action_id: u32,
    pub expected_information_gain_nats: f64,
    pub standard_error_nats: f64,
    pub replicates_per_model: usize,
    pub seed: u64,
    pub gain_per_cost: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSetAdequacyV1 {
    UniqueModel {
        family_id: u32,
        model_id: u32,
    },
    ParameterNotIdentified {
        family_id: u32,
        compatible_model_ids: Vec<u32>,
    },
    InFamilyUnresolved {
        compatible_family_ids: Vec<u32>,
        compatible_model_ids: Vec<u32>,
    },
    OutOfModelSet,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequentialDecisionV1 {
    CommitModel {
        model_id: u32,
        family_id: u32,
        posterior_probability: f64,
    },
    CommitFamilyParameterNotIdentified {
        family_id: u32,
        posterior_probability: f64,
        compatible_model_ids: Vec<u32>,
    },
    Continue {
        reason: &'static str,
    },
    AbstainOutOfModelSet,
    AbstainFamilyUnresolved {
        compatible_family_ids: Vec<u32>,
    },
    AbstainInsufficientConfidence {
        best_family_id: u32,
        posterior_probability: f64,
    },
    AbstainParameterNotIdentified {
        family_id: u32,
        compatible_model_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequentialPolicyConfigV1 {
    pub posterior_commit_threshold: f64,
    /// Campaign-level maximum of summed joint Mahalanobis residual per model.
    /// This must be calibrated separately on an independent stream.
    pub maximum_campaign_residual: f64,
    pub maximum_experiments: usize,
    pub maximum_total_cost: f64,
    pub eig_replicates_per_model: usize,
    pub eig_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialPolicyReportV1 {
    pub posterior: PosteriorReportV1,
    pub adequacy: ModelSetAdequacyV1,
    pub decision: SequentialDecisionV1,
    pub experiments_used: usize,
    pub total_cost: f64,
    pub remaining_action_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialEvidenceError {
    Experiment(ExcitationDesignError),
    Covariance(CovarianceError),
    InvalidPrior,
    PriorDoesNotMatchModelSet,
    DuplicateAction(u32),
    TooManyActions,
    UnknownAction(u32),
    UnknownModel(u32),
    InvalidActionCost,
    InvalidActionIdentity,
    InvalidActionPrediction,
    InvalidObservationShape,
    NonFiniteObservation,
    DuplicateObservationAction,
    InvalidResidualThreshold,
    InvalidCommitThreshold,
    InvalidCampaignCost,
    InvalidReplicateCount,
    ExcessiveReplicateCount,
    NoRemainingActions,
    PosteriorNormalizationFailed,
}

impl fmt::Display for SequentialEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Experiment(error) => write!(f, "sequential experiment rejected: {error}"),
            Self::Covariance(error) => write!(f, "sequential covariance rejected: {error}"),
            Self::InvalidPrior => write!(f, "model prior must be finite, positive and normalized"),
            Self::PriorDoesNotMatchModelSet => {
                write!(f, "prior must contain exactly one entry for every candidate model")
            }
            Self::DuplicateAction(id) => write!(f, "action ID {id} is repeated"),
            Self::TooManyActions => {
                write!(f, "action catalogue exceeds the bounded reference limit")
            },
            Self::UnknownAction(id) => write!(f, "observation references unknown action ID {id}"),
            Self::UnknownModel(id) => write!(f, "candidate model ID {id} is not registered"),
            Self::InvalidActionCost => write!(f, "action cost must be finite and positive"),
            Self::InvalidActionIdentity => {
                write!(f, "action covariance identity does not match its request")
            },
            Self::InvalidActionPrediction => {
                write!(f, "action predictions do not match candidate models")
            },
            Self::InvalidObservationShape => {
                write!(f, "candidate-visible trace shape/order/schedule is invalid")
            },
            Self::NonFiniteObservation => {
                write!(f, "candidate-visible observation contains a non-finite value")
            },
            Self::DuplicateObservationAction => {
                write!(f, "an action may appear at most once in this finite campaign")
            },
            Self::InvalidResidualThreshold => {
                write!(f, "campaign residual threshold must be finite and non-negative")
            },
            Self::InvalidCommitThreshold => {
                write!(f, "posterior commit threshold must lie strictly between zero and one")
            }
            Self::InvalidCampaignCost => {
                write!(f, "maximum campaign cost must be finite and positive")
            },
            Self::InvalidReplicateCount => {
                write!(f, "at least one EIG replicate per model is required")
            },
            Self::ExcessiveReplicateCount => {
                write!(f, "EIG replicate count exceeds the bounded reference limit")
            },
            Self::NoRemainingActions => write!(f, "no unused admissible actions remain"),
            Self::PosteriorNormalizationFailed => write!(f, "posterior log-normalization failed"),
        }
    }
}

impl std::error::Error for SequentialEvidenceError {}

/// Prepare one finite action by evaluating candidate-visible means once.
/// The expected request identity is provided separately from covariance metadata.
pub fn prepare_sequential_action(
    scenario: &ExperimentDesignScenario,
    action_id: u32,
    profile: InputProfile,
    selected_sensor_ids: Vec<u32>,
    covariance: ObservationCovarianceV1,
    expected_identity: ObservationDesignIdentityV1,
    cost: f64,
) -> Result<SequentialActionV1, SequentialEvidenceError> {
    scenario
        .validate()
        .map_err(SequentialEvidenceError::Experiment)?;
    validate_identity(
        scenario,
        &selected_sensor_ids,
        &covariance,
        &expected_identity,
    )?;
    if !cost.is_finite() || cost <= 0.0 {
        return Err(SequentialEvidenceError::InvalidActionCost);
    }
    let score = evaluate_input_profile(scenario, &profile, &selected_sensor_ids)
        .map_err(SequentialEvidenceError::Experiment)?;
    if score.selected_sensor_ids != selected_sensor_ids {
        return Err(SequentialEvidenceError::InvalidActionPrediction);
    }
    let mut model_means = score.model_observation_means;
    model_means.sort_by_key(|item| item.model_id);
    let mut expected_ids: Vec<u32> = scenario.models.iter().map(|model| model.id).collect();
    expected_ids.sort_unstable();
    let actual_ids: Vec<u32> = model_means.iter().map(|item| item.model_id).collect();
    if actual_ids != expected_ids
        || model_means.iter().any(|item| {
            item.observations.len() != scenario.observation_times.len()
                || item.observations.iter().any(|row| row.len() != selected_sensor_ids.len())
                || item.observations.iter().flatten().any(|value| !value.is_finite())
        })
    {
        return Err(SequentialEvidenceError::InvalidActionPrediction);
    }
    Ok(SequentialActionV1 {
        action_id,
        profile,
        selected_sensor_ids,
        covariance,
        expected_identity,
        cost,
        model_means,
    })
}

pub fn validate_action_catalogue(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
) -> Result<(), SequentialEvidenceError> {
    scenario
        .validate()
        .map_err(SequentialEvidenceError::Experiment)?;
    if actions.is_empty() {
        return Err(SequentialEvidenceError::NoRemainingActions);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(SequentialEvidenceError::TooManyActions);
    }
    let mut action_ids = HashSet::new();
    for action in actions {
        if !action_ids.insert(action.action_id) {
            return Err(SequentialEvidenceError::DuplicateAction(action.action_id));
        }
        if !action.cost.is_finite() || action.cost <= 0.0 {
            return Err(SequentialEvidenceError::InvalidActionCost);
        }
        validate_identity(
            scenario,
            &action.selected_sensor_ids,
            &action.covariance,
            &action.expected_identity,
        )?;
        let mut expected_ids: Vec<u32> = scenario.models.iter().map(|model| model.id).collect();
        expected_ids.sort_unstable();
        let mut actual_ids: Vec<u32> =
            action.model_means.iter().map(|item| item.model_id).collect();
        actual_ids.sort_unstable();
        if actual_ids != expected_ids
            || action.model_means.iter().any(|item| {
                item.observations.len() != scenario.observation_times.len()
                    || item
                        .observations
                        .iter()
                        .any(|row| row.len() != action.selected_sensor_ids.len())
                    || item.observations.iter().flatten().any(|value| !value.is_finite())
            })
        {
            return Err(SequentialEvidenceError::InvalidActionPrediction);
        }
    }
    validate_prior(scenario, prior)
}

pub fn posterior_after_history(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
) -> Result<PosteriorReportV1, SequentialEvidenceError> {
    validate_action_catalogue(scenario, actions, prior)?;
    posterior_after_history_validated(scenario, actions, prior, history)
}

fn posterior_after_history_validated(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
) -> Result<PosteriorReportV1, SequentialEvidenceError> {
    let mut log_weights = scenario
        .models
        .iter()
        .map(|model| {
            prior
                .iter()
                .find(|entry| entry.model_id == model.id)
                .map(|entry| entry.probability.ln())
                .ok_or(SequentialEvidenceError::PriorDoesNotMatchModelSet)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let action_lookup: HashMap<u32, &SequentialActionV1> =
        actions.iter().map(|action| (action.action_id, action)).collect();
    let mut seen_observations = HashSet::new();

    for observation in history {
        if !seen_observations.insert(observation.action_id) {
            return Err(SequentialEvidenceError::DuplicateObservationAction);
        }
        let action = action_lookup
            .get(&observation.action_id)
            .copied()
            .ok_or(SequentialEvidenceError::UnknownAction(observation.action_id))?;
        validate_observation(action, scenario, observation)?;
        for (index, model) in scenario.models.iter().enumerate() {
            let means = model_means_for(action, model.id)?;
            let residual = flatten_residual(&observation.values, &means.observations);
            // The Gaussian normalizer cancels across candidate models for this action.
            let log_likelihood_without_common_constant = -action
                .covariance
                .gaussian_quadratic_form_half(&residual)
                .map_err(SequentialEvidenceError::Covariance)?;
            log_weights[index] += log_likelihood_without_common_constant;
        }
    }
    posterior_from_log_weights(scenario, prior, &log_weights)
}

/// Estimate I(M;Y|history) using stratified Monte Carlo. Each model receives
/// the same number of draws; prior weights combine conditional entropies.
/// The seed and Monte Carlo standard error are returned with the estimate.
pub fn estimate_action_expected_information_gain(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    action_id: u32,
    replicates_per_model: usize,
    seed: u64,
) -> Result<ExpectedInformationGainV1, SequentialEvidenceError> {
    validate_action_catalogue(scenario, actions, prior)?;
    if replicates_per_model == 0 {
        return Err(SequentialEvidenceError::InvalidReplicateCount);
    }
    if replicates_per_model > MAX_EIG_REPLICATES_PER_MODEL {
        return Err(SequentialEvidenceError::ExcessiveReplicateCount);
    }
    let action = actions
        .iter()
        .find(|action| action.action_id == action_id)
        .ok_or(SequentialEvidenceError::UnknownAction(action_id))?;
    if history.iter().any(|entry| entry.action_id == action_id) {
        return Err(SequentialEvidenceError::DuplicateObservationAction);
    }
    let posterior = posterior_after_history(scenario, actions, prior, history)?;
    let prior_weights: HashMap<u32, f64> = posterior
        .weights
        .iter()
        .map(|entry| (entry.model_id, entry.probability))
        .collect();
    let prior_entropy = posterior.model_entropy_nats;
    let mut expected_entropy = 0.0;
    let mut variance_of_mean = 0.0;

    for model in &scenario.models {
        let model_probability = *prior_weights
            .get(&model.id)
            .ok_or(SequentialEvidenceError::UnknownModel(model.id))?;
        let means = model_means_for(action, model.id)?;
        let flattened_means = flatten_values(&means.observations);
        let mut rng = SplitMix64::new(seed_for(seed, action_id, model.id));
        let mut entropy_sum = 0.0;
        let mut entropy_squared_sum = 0.0;

        for _ in 0..replicates_per_model {
            let noise = action
                .covariance
                .covariance()
                .sample_gaussian(&mut rng)
                .map_err(SequentialEvidenceError::Covariance)?;
            let values = add_vectors(&flattened_means, &noise)?;
            let observation = ActionObservationV1 {
                action_id,
                sensor_ids: action.selected_sensor_ids.clone(),
                times: scenario.observation_times.clone(),
                values: unflatten_values(
                    &values,
                    scenario.observation_times.len(),
                    action.selected_sensor_ids.len(),
                )?,
            };
            let mut next_history = history.to_vec();
            next_history.push(observation);
            let updated =
                posterior_after_history_validated(scenario, actions, prior, &next_history)?;
            entropy_sum += updated.model_entropy_nats;
            entropy_squared_sum += updated.model_entropy_nats * updated.model_entropy_nats;
        }

        let n = replicates_per_model as f64;
        let conditional_mean = entropy_sum / n;
        let conditional_variance = if replicates_per_model > 1 {
            ((entropy_squared_sum - entropy_sum * entropy_sum / n) / (n - 1.0))
                .max(0.0)
        } else {
            0.0
        };
        expected_entropy += model_probability * conditional_mean;
        variance_of_mean += model_probability * model_probability * conditional_variance / n;
    }

    let gain = (prior_entropy - expected_entropy).max(0.0);
    let standard_error = variance_of_mean.sqrt();
    Ok(ExpectedInformationGainV1 {
        action_id,
        expected_information_gain_nats: gain,
        standard_error_nats: standard_error,
        replicates_per_model,
        seed,
        gain_per_cost: gain / action.cost,
    })
}

pub fn choose_greedy_information_action(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    replicates_per_model: usize,
    seed: u64,
) -> Result<ExpectedInformationGainV1, SequentialEvidenceError> {
    let used: HashSet<u32> = history.iter().map(|entry| entry.action_id).collect();
    let mut best: Option<ExpectedInformationGainV1> = None;
    for action in actions.iter().filter(|action| !used.contains(&action.action_id)) {
        let score = estimate_action_expected_information_gain(
            scenario,
            actions,
            prior,
            history,
            action.action_id,
            replicates_per_model,
            seed_for(seed, action.action_id, 0),
        )?;
        let replace = best.as_ref().is_none_or(|current| {
            score.gain_per_cost > current.gain_per_cost + TOLERANCE
                || ((score.gain_per_cost - current.gain_per_cost).abs() <= TOLERANCE
                    && score.action_id < current.action_id)
        });
        if replace {
            best = Some(score);
        }
    }
    best.ok_or(SequentialEvidenceError::NoRemainingActions)
}

pub fn assess_model_set_adequacy(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    maximum_campaign_residual: f64,
) -> Result<ModelSetAdequacyV1, SequentialEvidenceError> {
    validate_action_catalogue(scenario, actions, prior)?;
    if !maximum_campaign_residual.is_finite() || maximum_campaign_residual < 0.0 {
        return Err(SequentialEvidenceError::InvalidResidualThreshold);
    }
    if history.is_empty() {
        let mut families: Vec<u32> = prior.iter().map(|entry| entry.family_id).collect();
        families.sort_unstable();
        families.dedup();
        return Ok(ModelSetAdequacyV1::InFamilyUnresolved {
            compatible_family_ids: families,
            compatible_model_ids: scenario.models.iter().map(|model| model.id).collect(),
        });
    }
    let action_lookup: HashMap<u32, &SequentialActionV1> =
        actions.iter().map(|action| (action.action_id, action)).collect();
    let mut residuals = vec![0.0; scenario.models.len()];

    for observation in history {
        let action = action_lookup
            .get(&observation.action_id)
            .copied()
            .ok_or(SequentialEvidenceError::UnknownAction(observation.action_id))?;
        validate_observation(action, scenario, observation)?;
        for (index, model) in scenario.models.iter().enumerate() {
            let means = model_means_for(action, model.id)?;
            let residual = flatten_residual(&observation.values, &means.observations);
            residuals[index] += action
                .covariance
                .gaussian_quadratic_form_half(&residual)
                .map_err(SequentialEvidenceError::Covariance)?;
        }
    }

    let compatible: Vec<u32> = scenario
        .models
        .iter()
        .zip(residuals.iter())
        .filter_map(|(model, residual)| {
            (*residual <= maximum_campaign_residual).then_some(model.id)
        })
        .collect();
    if compatible.is_empty() {
        return Ok(ModelSetAdequacyV1::OutOfModelSet);
    }
    if compatible.len() == 1 {
        let model_id = compatible[0];
        return Ok(ModelSetAdequacyV1::UniqueModel {
            family_id: family_for(prior, model_id)?,
            model_id,
        });
    }

    let mut families: Vec<u32> = compatible
        .iter()
        .map(|model_id| family_for(prior, *model_id))
        .collect::<Result<HashSet<_>, _>>()?
        .into_iter()
        .collect();
    families.sort_unstable();
    let mut compatible_model_ids = compatible;
    compatible_model_ids.sort_unstable();
    if families.len() == 1 {
        Ok(ModelSetAdequacyV1::ParameterNotIdentified {
            family_id: families[0],
            compatible_model_ids,
        })
    } else {
        Ok(ModelSetAdequacyV1::InFamilyUnresolved {
            compatible_family_ids: families,
            compatible_model_ids,
        })
    }
}

pub fn decide_sequential_stop(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    config: SequentialPolicyConfigV1,
) -> Result<SequentialPolicyReportV1, SequentialEvidenceError> {
    validate_policy_config(config)?;
    validate_action_catalogue(scenario, actions, prior)?;
    let posterior = posterior_after_history(scenario, actions, prior, history)?;
    let adequacy = assess_model_set_adequacy(
        scenario,
        actions,
        prior,
        history,
        config.maximum_campaign_residual,
    )?;
    let used: HashSet<u32> = history.iter().map(|entry| entry.action_id).collect();
    let remaining_action_ids: Vec<u32> = actions
        .iter()
        .filter(|action| {
            !used.contains(&action.action_id)
                && total_cost + action.cost <= config.maximum_total_cost + TOLERANCE
        })
        .map(|action| action.action_id)
        .collect();
    let total_cost: f64 = history
        .iter()
        .map(|observation| {
            actions
                .iter()
                .find(|action| action.action_id == observation.action_id)
                .map(|action| action.cost)
                .ok_or(SequentialEvidenceError::UnknownAction(observation.action_id))
        })
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum();
    if history.len() > config.maximum_experiments {
        return Err(SequentialEvidenceError::InvalidObservationShape);
    }
    if !config.maximum_total_cost.is_finite() || config.maximum_total_cost <= 0.0 {
        return Err(SequentialEvidenceError::InvalidCampaignCost);
    }
    if total_cost > config.maximum_total_cost + TOLERANCE {
        return Err(SequentialEvidenceError::InvalidCampaignCost);
    }
    let family_probabilities: HashMap<u32, f64> =
        posterior.family_probabilities.iter().copied().collect();
    let best_family = posterior
        .family_probabilities
        .iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .copied()
        .ok_or(SequentialEvidenceError::PosteriorNormalizationFailed)?;

    let decision = match &adequacy {
        ModelSetAdequacyV1::OutOfModelSet => SequentialDecisionV1::AbstainOutOfModelSet,
        ModelSetAdequacyV1::InFamilyUnresolved {
            compatible_family_ids,
            ..
        } => {
            if history.len() < config.maximum_experiments && !remaining_action_ids.is_empty() {
                SequentialDecisionV1::Continue {
                    reason: "multiple structural families remain adequate",
                }
            } else {
                SequentialDecisionV1::AbstainFamilyUnresolved {
                    compatible_family_ids: compatible_family_ids.clone(),
                }
            }
        }
        ModelSetAdequacyV1::ParameterNotIdentified {
            family_id,
            compatible_model_ids,
        } => {
            let family_probability = *family_probabilities.get(family_id).unwrap_or(&0.0);
            if family_probability >= config.posterior_commit_threshold {
                SequentialDecisionV1::CommitFamilyParameterNotIdentified {
                    family_id: *family_id,
                    posterior_probability: family_probability,
                    compatible_model_ids: compatible_model_ids.clone(),
                }
            } else if history.len() < config.maximum_experiments
                && !remaining_action_ids.is_empty()
            {
                SequentialDecisionV1::Continue {
                    reason: "family posterior is below the commit threshold",
                }
            } else {
                SequentialDecisionV1::AbstainParameterNotIdentified {
                    family_id: *family_id,
                    compatible_model_ids: compatible_model_ids.clone(),
                }
            }
        }
        ModelSetAdequacyV1::UniqueModel {
            family_id,
            model_id,
        } => {
            let probability = posterior
                .weights
                .iter()
                .find(|entry| entry.model_id == *model_id)
                .map(|entry| entry.probability)
                .unwrap_or(0.0);
            if probability >= config.posterior_commit_threshold {
                SequentialDecisionV1::CommitModel {
                    model_id: *model_id,
                    family_id: *family_id,
                    posterior_probability: probability,
                }
            } else if history.len() < config.maximum_experiments
                && !remaining_action_ids.is_empty()
            {
                SequentialDecisionV1::Continue {
                    reason: "candidate posterior is below the commit threshold",
                }
            } else {
                SequentialDecisionV1::AbstainInsufficientConfidence {
                    best_family_id: best_family.0,
                    posterior_probability: best_family.1,
                }
            }
        }
    };

    Ok(SequentialPolicyReportV1 {
        posterior,
        adequacy,
        decision,
        experiments_used: history.len(),
        total_cost,
        remaining_action_ids,
    })
}

fn validate_policy_config(
    config: SequentialPolicyConfigV1,
) -> Result<(), SequentialEvidenceError> {
    if !config.posterior_commit_threshold.is_finite()
        || config.posterior_commit_threshold <= 0.0
        || config.posterior_commit_threshold >= 1.0
    {
        return Err(SequentialEvidenceError::InvalidCommitThreshold);
    }
    if !config.maximum_campaign_residual.is_finite()
        || config.maximum_campaign_residual < 0.0
    {
        return Err(SequentialEvidenceError::InvalidResidualThreshold);
    }
    if config.maximum_experiments == 0 {
        return Err(SequentialEvidenceError::InvalidReplicateCount);
    }
    if !config.maximum_total_cost.is_finite() || config.maximum_total_cost <= 0.0 {
        return Err(SequentialEvidenceError::InvalidCampaignCost);
    }
    if config.eig_replicates_per_model == 0 {
        return Err(SequentialEvidenceError::InvalidReplicateCount);
    }
    if config.eig_replicates_per_model > MAX_EIG_REPLICATES_PER_MODEL {
        return Err(SequentialEvidenceError::ExcessiveReplicateCount);
    }
    Ok(())
}

fn validate_prior(
    scenario: &ExperimentDesignScenario,
    prior: &[ModelPriorV1],
) -> Result<(), SequentialEvidenceError> {
    if prior.len() != scenario.models.len() {
        return Err(SequentialEvidenceError::PriorDoesNotMatchModelSet);
    }
    let mut ids = HashSet::new();
    let mut sum = 0.0;
    for entry in prior {
        if !ids.insert(entry.model_id)
            || !entry.probability.is_finite()
            || entry.probability <= 0.0
        {
            return Err(SequentialEvidenceError::InvalidPrior);
        }
        if !scenario.models.iter().any(|model| model.id == entry.model_id) {
            return Err(SequentialEvidenceError::UnknownModel(entry.model_id));
        }
        sum += entry.probability;
    }
    if (sum - 1.0).abs() > TOLERANCE {
        return Err(SequentialEvidenceError::InvalidPrior);
    }
    for model in &scenario.models {
        if !ids.contains(&model.id) {
            return Err(SequentialEvidenceError::PriorDoesNotMatchModelSet);
        }
    }
    Ok(())
}

fn validate_identity(
    scenario: &ExperimentDesignScenario,
    selected_sensor_ids: &[u32],
    covariance: &ObservationCovarianceV1,
    identity: &ObservationDesignIdentityV1,
) -> Result<(), SequentialEvidenceError> {
    let expected_dimension = scenario
        .observation_times
        .len()
        .checked_mul(selected_sensor_ids.len())
        .ok_or(SequentialEvidenceError::InvalidActionIdentity)?;
    if identity.covariance_id.trim().is_empty()
        || identity.observation_process_id.trim().is_empty()
        || identity.sampling_policy_id.trim().is_empty()
        || identity.observation_operator_id.trim().is_empty()
        || identity.availability_cutoff_id.trim().is_empty()
        || covariance.covariance_id != identity.covariance_id
        || covariance.observation_process_id != identity.observation_process_id
        || covariance.sampling_policy_id != identity.sampling_policy_id
        || covariance.observation_operator_id != identity.observation_operator_id
        || covariance.availability_cutoff_id != identity.availability_cutoff_id
        || covariance.sensor_ids != selected_sensor_ids
        || covariance.times != scenario.observation_times
        || covariance.covariance().dimension() != expected_dimension
    {
        return Err(SequentialEvidenceError::InvalidActionIdentity);
    }
    Ok(())
}

fn validate_observation(
    action: &SequentialActionV1,
    scenario: &ExperimentDesignScenario,
    observation: &ActionObservationV1,
) -> Result<(), SequentialEvidenceError> {
    if observation.sensor_ids != action.selected_sensor_ids
        || observation.times != scenario.observation_times
        || observation.values.len() != observation.times.len()
        || observation.values.iter().any(|row| row.len() != observation.sensor_ids.len())
    {
        return Err(SequentialEvidenceError::InvalidObservationShape);
    }
    if observation.values.iter().flatten().any(|value| !value.is_finite()) {
        return Err(SequentialEvidenceError::NonFiniteObservation);
    }
    Ok(())
}

fn model_means_for(
    action: &SequentialActionV1,
    model_id: u32,
) -> Result<&ModelObservationMeans, SequentialEvidenceError> {
    action
        .model_means
        .iter()
        .find(|means| means.model_id == model_id)
        .ok_or(SequentialEvidenceError::UnknownModel(model_id))
}

fn family_for(prior: &[ModelPriorV1], model_id: u32) -> Result<u32, SequentialEvidenceError> {
    prior
        .iter()
        .find(|entry| entry.model_id == model_id)
        .map(|entry| entry.family_id)
        .ok_or(SequentialEvidenceError::UnknownModel(model_id))
}

fn flatten_values(values: &[Vec<f64>]) -> Vec<f64> {
    values.iter().flatten().copied().collect()
}

fn flatten_residual(observed: &[Vec<f64>], expected: &[Vec<f64>]) -> Vec<f64> {
    observed
        .iter()
        .flatten()
        .zip(expected.iter().flatten())
        .map(|(observed, expected)| observed - expected)
        .collect()
}

fn add_vectors(left: &[f64], right: &[f64]) -> Result<Vec<f64>, SequentialEvidenceError> {
    if left.len() != right.len() {
        return Err(SequentialEvidenceError::InvalidObservationShape);
    }
    let values: Vec<f64> = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left + right)
        .collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SequentialEvidenceError::NonFiniteObservation);
    }
    Ok(values)
}

fn unflatten_values(
    values: &[f64],
    time_count: usize,
    sensor_count: usize,
) -> Result<Vec<Vec<f64>>, SequentialEvidenceError> {
    if time_count.checked_mul(sensor_count) != Some(values.len()) || sensor_count == 0 {
        return Err(SequentialEvidenceError::InvalidObservationShape);
    }
    Ok(values.chunks(sensor_count).map(|row| row.to_vec()).collect())
}

fn posterior_from_log_weights(
    scenario: &ExperimentDesignScenario,
    prior: &[ModelPriorV1],
    log_weights: &[f64],
) -> Result<PosteriorReportV1, SequentialEvidenceError> {
    if log_weights.len() != scenario.models.len()
        || log_weights.iter().any(|value| !value.is_finite())
    {
        return Err(SequentialEvidenceError::PosteriorNormalizationFailed);
    }
    let maximum = log_weights
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let unnormalized: Vec<f64> = log_weights
        .iter()
        .map(|value| (value - maximum).exp())
        .collect();
    let normalizer: f64 = unnormalized.iter().sum();
    if !normalizer.is_finite() || normalizer <= 0.0 {
        return Err(SequentialEvidenceError::PosteriorNormalizationFailed);
    }
    let weights: Vec<PosteriorWeightV1> = scenario
        .models
        .iter()
        .zip(unnormalized.iter())
        .map(|(model, weight)| {
            Ok(PosteriorWeightV1 {
                model_id: model.id,
                family_id: family_for(prior, model.id)?,
                probability: weight / normalizer,
            })
        })
        .collect::<Result<Vec<_>, SequentialEvidenceError>>()?;
    let mut family_map: HashMap<u32, f64> = HashMap::new();
    for weight in &weights {
        *family_map.entry(weight.family_id).or_insert(0.0) += weight.probability;
    }
    let mut family_probabilities: Vec<(u32, f64)> = family_map.into_iter().collect();
    family_probabilities.sort_by_key(|entry| entry.0);
    Ok(PosteriorReportV1 {
        model_entropy_nats: entropy(weights.iter().map(|entry| entry.probability)),
        family_entropy_nats: entropy(family_probabilities.iter().map(|entry| entry.1)),
        weights,
        family_probabilities,
    })
}

fn entropy(probabilities: impl Iterator<Item = f64>) -> f64 {
    probabilities
        .filter(|probability| *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum()
}

fn seed_for(seed: u64, action_id: u32, model_id: u32) -> u64 {
    seed ^ (u64::from(action_id) << 32)
        ^ u64::from(model_id).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{ForcingSegment, ModelCandidate};
    use crate::network::{Edge, NetworkScenario};
    use crate::noise_robustness::{Ar1KroneckerCovariance, DenseSpdCovariance};
    use crate::sensor_design::SensorCandidate;

    fn scenario() -> ExperimentDesignScenario {
        let network = |bypass: bool| {
            let mut edges = vec![
                Edge { left: 0, right: 1, conductance: 1.0 },
                Edge { left: 1, right: 2, conductance: 1.0 },
            ];
            if bypass {
                edges.push(Edge { left: 0, right: 2, conductance: 0.4 });
            }
            NetworkScenario {
                name: if bypass { "sequential-bypass" } else { "sequential-chain" },
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![0.2, 0.0, 0.0],
                edges,
                horizon: 2.0,
            }
        };
        ExperimentDesignScenario {
            name: "sequential-chain-bypass",
            models: vec![
                ModelCandidate { id: 1, label: "chain", network: network(false) },
                ModelCandidate { id: 2, label: "bypass", network: network(true) },
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

    fn prior() -> Vec<ModelPriorV1> {
        vec![
            ModelPriorV1 { model_id: 1, family_id: 100, probability: 0.5 },
            ModelPriorV1 { model_id: 2, family_id: 200, probability: 0.5 },
        ]
    }

    fn action(
        scenario: &ExperimentDesignScenario,
        action_id: u32,
        node: usize,
        amplitude: f64,
    ) -> SequentialActionV1 {
        let profile = InputProfile {
            id: action_id,
            label: "single-pulse-action",
            segments: vec![
                ForcingSegment {
                    start_time: 0.0,
                    end_time: 0.5,
                    forcing: {
                        let mut values = vec![0.0; 3];
                        values[node] = amplitude;
                        values
                    },
                },
                ForcingSegment {
                    start_time: 0.5,
                    end_time: 2.0,
                    forcing: vec![0.0, 0.0, 0.0],
                },
            ],
        };
        let covariance_id = format!("covariance:sequential-action-{action_id}");
        let process_id = "process:ar1-rho-0.6".to_owned();
        let sampling_id = "sampling:uniform-five".to_owned();
        let operator_id = "operator:sensors-10-11".to_owned();
        let cutoff_id = "availability:all-five".to_owned();
        let covariance = ObservationCovarianceV1::new(
            covariance_id.clone(),
            process_id.clone(),
            sampling_id.clone(),
            operator_id.clone(),
            cutoff_id.clone(),
            vec![10, 11],
            scenario.observation_times.clone(),
            Ar1KroneckerCovariance::new(
                scenario.observation_times.len(),
                0.6,
                DenseSpdCovariance::new(scenario.noise_covariance.clone()).unwrap(),
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
            scenario,
            action_id,
            profile,
            vec![10, 11],
            covariance,
            identity,
            1.0,
        )
        .unwrap()
    }

    fn config() -> SequentialPolicyConfigV1 {
        SequentialPolicyConfigV1 {
            posterior_commit_threshold: 0.95,
            maximum_campaign_residual: 40.0,
            maximum_experiments: 2,
            maximum_total_cost: 2.0,
            eig_replicates_per_model: 128,
            eig_seed: 101,
        }
    }

    #[test]
    fn posterior_update_is_deterministic_and_changes_with_trace() {
        let scenario = scenario();
        let actions = vec![action(&scenario, 1, 0, 1.0), action(&scenario, 2, 2, -1.0)];
        let observation = ActionObservationV1 {
            action_id: 1,
            sensor_ids: vec![10, 11],
            times: scenario.observation_times.clone(),
            values: actions[0].model_means[0].observations.clone(),
        };
        let first =
            posterior_after_history(&scenario, &actions, &prior(), &[observation.clone()])
                .unwrap();
        let second =
            posterior_after_history(&scenario, &actions, &prior(), &[observation]).unwrap();
        assert_eq!(first, second);
        assert!(first.weights[0].probability > first.weights[1].probability);
        assert!(
            (first
                .weights
                .iter()
                .map(|entry| entry.probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn expected_information_gain_is_seeded_and_nonnegative() {
        let scenario = scenario();
        let actions = vec![action(&scenario, 1, 0, 1.0), action(&scenario, 2, 2, -1.0)];
        let first = estimate_action_expected_information_gain(
            &scenario, &actions, &prior(), &[], 1, 128, 55,
        )
        .unwrap();
        let second = estimate_action_expected_information_gain(
            &scenario, &actions, &prior(), &[], 1, 128, 55,
        )
        .unwrap();
        assert_eq!(first, second);
        assert!(first.expected_information_gain_nats >= 0.0);
        assert!(first.standard_error_nats.is_finite());
    }

    #[test]
    fn out_of_model_set_never_commits_a_candidate() {
        let scenario = scenario();
        let actions = vec![action(&scenario, 1, 0, 1.0), action(&scenario, 2, 2, -1.0)];
        let history = vec![ActionObservationV1 {
            action_id: 1,
            sensor_ids: vec![10, 11],
            times: scenario.observation_times.clone(),
            values: vec![vec![100.0, -100.0]; scenario.observation_times.len()],
        }];
        let report =
            decide_sequential_stop(&scenario, &actions, &prior(), &history, config()).unwrap();
        assert_eq!(report.adequacy, ModelSetAdequacyV1::OutOfModelSet);
        assert_eq!(report.decision, SequentialDecisionV1::AbstainOutOfModelSet);
    }

    #[test]
    fn unresolved_candidate_family_does_not_force_a_model_winner() {
        let scenario = scenario();
        let mut same_family_prior = prior();
        same_family_prior[1].family_id = 100;
        let actions = vec![action(&scenario, 1, 0, 1.0)];
        let means = &actions[0].model_means[0].observations;
        let history = vec![ActionObservationV1 {
            action_id: 1,
            sensor_ids: vec![10, 11],
            times: scenario.observation_times.clone(),
            values: means.clone(),
        }];
        let report = decide_sequential_stop(
            &scenario,
            &actions,
            &same_family_prior,
            &history,
            config(),
        )
        .unwrap();
        assert!(matches!(
            report.adequacy,
            ModelSetAdequacyV1::ParameterNotIdentified { .. }
        ));
        assert!(matches!(
            report.decision,
            SequentialDecisionV1::CommitFamilyParameterNotIdentified { .. }
                | SequentialDecisionV1::AbstainParameterNotIdentified { .. }
        ));
    }

    #[test]
    fn action_identity_mismatch_rejects_before_posterior_update() {
        let scenario = scenario();
        let mut action = action(&scenario, 1, 0, 1.0);
        action.expected_identity.observation_process_id = "stale".to_owned();
        assert_eq!(
            validate_action_catalogue(&scenario, &[action], &prior()),
            Err(SequentialEvidenceError::InvalidActionIdentity)
        );
    }

    #[test]
    fn duplicate_action_observation_is_rejected() {
        let scenario = scenario();
        let actions = vec![action(&scenario, 1, 0, 1.0)];
        let observation = ActionObservationV1 {
            action_id: 1,
            sensor_ids: vec![10, 11],
            times: scenario.observation_times.clone(),
            values: actions[0].model_means[0].observations.clone(),
        };
        assert_eq!(
            posterior_after_history(
                &scenario,
                &actions,
                &prior(),
                &[observation.clone(), observation],
            ),
            Err(SequentialEvidenceError::DuplicateObservationAction)
        );
    }
}
