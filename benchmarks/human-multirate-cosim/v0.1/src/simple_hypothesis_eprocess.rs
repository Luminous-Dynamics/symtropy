//! Anytime-valid likelihood-ratio e-process for two fully specified models.
//!
//! Under each simple null, the action-conditional likelihood ratio is a
//! non-negative martingale when actions are chosen predictably from the
//! candidate-visible past and each observation follows its identity-bound,
//! known Gaussian covariance. Bonferroni allocates half the campaign alpha
//! to each directional test.

use crate::experiment_design::{ExcitationDesignError, ModelObservationMeans};
use crate::noise_robustness::CovarianceError;
use crate::sequential_evidence::{
    validate_action_catalogue, ActionObservationV1, ModelPriorV1,
    SequentialActionV1, SequentialEvidenceError,
};
use crate::experiment_design::ExperimentDesignScenario;
use std::collections::HashSet;
use std::fmt;

const ALPHA_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleHypothesisEProcessDecisionV1 {
    SupportNull,
    SupportAlternative,
    InsufficientEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleHypothesisEProcessPrefixV1 {
    pub action_id: u32,
    /// log p_1(y_t | action_t) - log p_0(y_t | action_t)
    pub log_likelihood_ratio_increment: f64,
    pub cumulative_log_e_alternative_over_null: f64,
    pub cumulative_log_e_null_over_alternative: f64,
    pub decision: SimpleHypothesisEProcessDecisionV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleHypothesisEProcessReportV1 {
    pub null_model_id: u32,
    pub alternative_model_id: u32,
    pub total_campaign_alpha: f64,
    pub per_direction_alpha: f64,
    pub log_crossing_threshold: f64,
    pub directional_false_commit_bound: f64,
    pub two_direction_familywise_bound: f64,
    pub prefixes: Vec<SimpleHypothesisEProcessPrefixV1>,
    pub final_decision: SimpleHypothesisEProcessDecisionV1,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleHypothesisEProcessError {
    Sequential(SequentialEvidenceError),
    Covariance(CovarianceError),
    InvalidModelPair,
    UnknownModel(u32),
    InvalidAlpha,
    DuplicateActionObservation,
    UnknownAction(u32),
    InvalidObservationShape,
    NonFiniteObservation,
    NonFiniteLikelihoodRatio,
}

impl fmt::Display for SimpleHypothesisEProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sequential(error) => {
                write!(f, "simple-hypothesis e-process rejected: {error}")
            }
            Self::Covariance(error) => {
                write!(f, "simple-hypothesis covariance rejected: {error}")
            }
            Self::InvalidModelPair => {
                write!(f, "distinct null and alternative model IDs are required")
            }
            Self::UnknownModel(id) => write!(f, "hypothesis model ID {id} is not registered"),
            Self::InvalidAlpha => {
                write!(f, "campaign alpha must be finite and lie strictly between zero and one")
            }
            Self::DuplicateActionObservation => {
                write!(f, "an action observation may occur only once in this campaign")
            }
            Self::UnknownAction(id) => write!(f, "observation references unknown action {id}"),
            Self::InvalidObservationShape => {
                write!(f, "observation shape, sensor order or schedule is invalid")
            }
            Self::NonFiniteObservation => {
                write!(f, "observation contains a non-finite value")
            }
            Self::NonFiniteLikelihoodRatio => {
                write!(f, "Gaussian log-likelihood ratio is non-finite")
            }
        }
    }
}

impl std::error::Error for SimpleHypothesisEProcessError {}

/// Replay the two directional likelihood-ratio e-processes over an ordered
/// sequence of completed actions.
///
/// Candidate prior weights are accepted only so the existing action catalogue
/// validator can verify the candidate set. They do not enter either
/// likelihood-ratio process. Model predictions and action covariances must be
/// fixed before the corresponding observation is revealed.
pub fn evaluate_simple_hypothesis_eprocess(
    scenario: &ExperimentDesignScenario,
    actions: &[SequentialActionV1],
    candidate_prior: &[ModelPriorV1],
    history: &[ActionObservationV1],
    null_model_id: u32,
    alternative_model_id: u32,
    total_campaign_alpha: f64,
) -> Result<SimpleHypothesisEProcessReportV1, SimpleHypothesisEProcessError> {
    validate_action_catalogue(scenario, actions, candidate_prior)
        .map_err(SimpleHypothesisEProcessError::Sequential)?;
    if null_model_id == alternative_model_id {
        return Err(SimpleHypothesisEProcessError::InvalidModelPair);
    }
    for model_id in [null_model_id, alternative_model_id] {
        if !scenario.models.iter().any(|model| model.id == model_id) {
            return Err(SimpleHypothesisEProcessError::UnknownModel(model_id));
        }
    }
    if !total_campaign_alpha.is_finite()
        || total_campaign_alpha <= 0.0
        || total_campaign_alpha >= 1.0
    {
        return Err(SimpleHypothesisEProcessError::InvalidAlpha);
    }

    let per_direction_alpha = total_campaign_alpha * 0.5;
    let log_crossing_threshold = (1.0 / per_direction_alpha).ln();
    let action_lookup: std::collections::HashMap<u32, &SequentialActionV1> =
        actions.iter().map(|action| (action.action_id(), action)).collect();
    let mut seen_actions = HashSet::new();
    let mut cumulative_log_e_alternative_over_null = 0.0;
    let mut prefixes = Vec::with_capacity(history.len());

    for observation in history {
        if !seen_actions.insert(observation.action_id) {
            return Err(SimpleHypothesisEProcessError::DuplicateActionObservation);
        }
        let action = action_lookup
            .get(&observation.action_id)
            .copied()
            .ok_or(SimpleHypothesisEProcessError::UnknownAction(
                observation.action_id,
            ))?;
        validate_observation(action, scenario, observation)?;

        let null_means = action
            .model_means_for(null_model_id)
            .ok_or(SimpleHypothesisEProcessError::UnknownModel(null_model_id))?;
        let alternative_means = action
            .model_means_for(alternative_model_id)
            .ok_or(SimpleHypothesisEProcessError::UnknownModel(
                alternative_model_id,
            ))?;
        let null_residual =
            flatten_residual(&observation.values, &null_means.observations);
        let alternative_residual =
            flatten_residual(&observation.values, &alternative_means.observations);
        let null_quadratic = action
            .covariance()
            .gaussian_quadratic_form_half(&null_residual)
            .map_err(SimpleHypothesisEProcessError::Covariance)?;
        let alternative_quadratic = action
            .covariance()
            .gaussian_quadratic_form_half(&alternative_residual)
            .map_err(SimpleHypothesisEProcessError::Covariance)?;

        // log f_1(y|a) - log f_0(y|a) = q_0(y) - q_1(y).
        // The log normalizer cancels because both models share this action's
        // exact observation covariance.
        let log_likelihood_ratio_increment = null_quadratic - alternative_quadratic;
        if !log_likelihood_ratio_increment.is_finite() {
            return Err(SimpleHypothesisEProcessError::NonFiniteLikelihoodRatio);
        }
        cumulative_log_e_alternative_over_null += log_likelihood_ratio_increment;
        let cumulative_log_e_null_over_alternative =
            -cumulative_log_e_alternative_over_null;
        let decision = decision_from_log_evalues(
            cumulative_log_e_alternative_over_null,
            cumulative_log_e_null_over_alternative,
            log_crossing_threshold,
        );
        prefixes.push(SimpleHypothesisEProcessPrefixV1 {
            action_id: observation.action_id,
            log_likelihood_ratio_increment,
            cumulative_log_e_alternative_over_null,
            cumulative_log_e_null_over_alternative,
            decision,
        });
    }

    let final_decision = prefixes
        .last()
        .map(|prefix| prefix.decision)
        .unwrap_or(SimpleHypothesisEProcessDecisionV1::InsufficientEvidence);
    Ok(SimpleHypothesisEProcessReportV1 {
        null_model_id,
        alternative_model_id,
        total_campaign_alpha,
        per_direction_alpha,
        log_crossing_threshold,
        directional_false_commit_bound: per_direction_alpha,
        two_direction_familywise_bound: total_campaign_alpha,
        prefixes,
        final_decision,
        assumptions: vec![
            "exactly two fully specified simple candidate models",
            "each action is selected predictably before its current observation is revealed",
            "the action-specific Gaussian covariance is known and identity-bound",
            "action observation noise is conditionally independent of earlier action noise",
            "the same action covariance applies to null and alternative predictions",
            "no nuisance-parameter fitting, covariance estimation or model discrepancy",
            "Ville bound applies to the likelihood-ratio martingale under the corresponding simple null",
            "Bonferroni allocates half the total alpha to each direction",
            "held-out models and composite families are outside this guarantee",
        ],
    })
}

fn decision_from_log_evalues(
    log_e_alternative_over_null: f64,
    log_e_null_over_alternative: f64,
    threshold: f64,
) -> SimpleHypothesisEProcessDecisionV1 {
    if log_e_alternative_over_null >= threshold {
        SimpleHypothesisEProcessDecisionV1::SupportAlternative
    } else if log_e_null_over_alternative >= threshold {
        SimpleHypothesisEProcessDecisionV1::SupportNull
    } else {
        SimpleHypothesisEProcessDecisionV1::InsufficientEvidence
    }
}

fn validate_observation(
    action: &SequentialActionV1,
    scenario: &ExperimentDesignScenario,
    observation: &ActionObservationV1,
) -> Result<(), SimpleHypothesisEProcessError> {
    if observation.sensor_ids != action.selected_sensor_ids()
        || observation.times != scenario.observation_times
        || observation.values.len() != scenario.observation_times.len()
        || observation
            .values
            .iter()
            .any(|row| row.len() != action.selected_sensor_ids().len())
    {
        return Err(SimpleHypothesisEProcessError::InvalidObservationShape);
    }
    if observation.values.iter().flatten().any(|value| !value.is_finite()) {
        return Err(SimpleHypothesisEProcessError::NonFiniteObservation);
    }
    Ok(())
}

fn flatten_residual(observed: &[Vec<f64>], means: &[Vec<f64>]) -> Vec<f64> {
    observed
        .iter()
        .flatten()
        .zip(means.iter().flatten())
        .map(|(observed, mean)| observed - mean)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{ForcingSegment, InputProfile, ModelCandidate};
    use crate::network::{Edge, NetworkScenario};
    use crate::noise_robustness::{
        Ar1KroneckerCovariance, DenseSpdCovariance, ObservationCovarianceV1,
    };
    use crate::sensor_design::SensorCandidate;
    use crate::sequential_evidence::{
        prepare_sequential_action, ModelPriorV1,
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
                name: if bypass { "eprocess-bypass" } else { "eprocess-chain" },
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![0.2, 0.0, 0.0],
                edges,
                horizon: 2.0,
            }
        };
        ExperimentDesignScenario {
            name: "eprocess-chain-bypass",
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

    fn prior() -> Vec<ModelPriorV1> {
        vec![
            ModelPriorV1 { model_id: 1, family_id: 100, probability: 0.5 },
            ModelPriorV1 { model_id: 2, family_id: 200, probability: 0.5 },
        ]
    }

    fn action(fixture: &ExperimentDesignScenario, action_id: u32, node: usize, amplitude: f64) -> SequentialActionV1 {
        let mut pulse = vec![0.0; 3];
        pulse[node] = amplitude;
        let profile = InputProfile {
            id: action_id,
            label: "eprocess-pulse",
            segments: vec![
                ForcingSegment {
                    start_time: 0.0,
                    end_time: 0.5,
                    forcing: pulse,
                },
                ForcingSegment {
                    start_time: 0.5,
                    end_time: 2.0,
                    forcing: vec![0.0, 0.0, 0.0],
                },
            ],
        };
        let covariance_id = format!("covariance:eprocess-{action_id}");
        let process_id = "process:eprocess-ar1-rho-0.6".to_owned();
        let sampling_id = "sampling:eprocess-uniform-five".to_owned();
        let operator_id = "operator:eprocess-sensors".to_owned();
        let cutoff_id = "availability:eprocess-all-observations".to_owned();
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
            action_id,
            profile,
            vec![10, 11],
            covariance,
            identity,
            1.0,
        )
        .unwrap()
    }

    fn trace(action: &SequentialActionV1, model_id: u32, scenario: &ExperimentDesignScenario) -> ActionObservationV1 {
        let means = action.model_means_for(model_id).unwrap();
        ActionObservationV1 {
            action_id: action.action_id(),
            sensor_ids: action.selected_sensor_ids().to_vec(),
            times: scenario.observation_times.clone(),
            values: means.observations.clone(),
        }
    }

    #[test]
    fn gaussian_likelihood_ratio_uses_the_action_specific_full_covariance() {
        let fixture = scenario();
        let action = action(&fixture, 1, 0, 1.0);
        let observation = trace(&action, 1, &fixture);
        let report = evaluate_simple_hypothesis_eprocess(
            &fixture,
            &[action],
            &prior(),
            &[observation],
            1,
            2,
            0.05,
        )
        .unwrap();
        let prefix = &report.prefixes[0];
        assert!(prefix.log_likelihood_ratio_increment < 0.0);
        assert!((prefix.cumulative_log_e_null_over_alternative
            + prefix.cumulative_log_e_alternative_over_null).abs() < 1.0e-12);
        assert!((report.log_crossing_threshold - (40.0_f64).ln()).abs() < 1.0e-12);
        assert_eq!(report.directional_false_commit_bound, 0.025);
        assert_eq!(report.two_direction_familywise_bound, 0.05);
    }

    #[test]
    fn action_order_permutation_preserves_cumulative_log_evalue() {
        let fixture = scenario();
        let first = action(&fixture, 1, 0, 1.0);
        let second = action(&fixture, 2, 2, -1.0);
        let history_a = vec![
            trace(&first, 1, &fixture),
            trace(&second, 1, &fixture),
        ];
        let history_b = vec![
            trace(&second, 1, &fixture),
            trace(&first, 1, &fixture),
        ];
        let report_a = evaluate_simple_hypothesis_eprocess(
            &fixture,
            &[first.clone(), second.clone()],
            &prior(),
            &history_a,
            1,
            2,
            0.05,
        )
        .unwrap();
        let report_b = evaluate_simple_hypothesis_eprocess(
            &fixture,
            &[first, second],
            &prior(),
            &history_b,
            1,
            2,
            0.05,
        )
        .unwrap();
        assert!(
            (report_a.prefixes[1].cumulative_log_e_alternative_over_null
                - report_b.prefixes[1].cumulative_log_e_alternative_over_null)
                .abs()
                < 1.0e-10
        );
    }

    #[test]
    fn duplicate_action_trace_is_rejected() {
        let fixture = scenario();
        let action = action(&fixture, 1, 0, 1.0);
        let observation = trace(&action, 1, &fixture);
        assert_eq!(
            evaluate_simple_hypothesis_eprocess(
                &fixture,
                &[action],
                &prior(),
                &[observation.clone(), observation],
                1,
                2,
                0.05,
            ),
            Err(SimpleHypothesisEProcessError::DuplicateActionObservation)
        );
    }

    #[test]
    fn invalid_alpha_and_hypothesis_pair_reject() {
        let fixture = scenario();
        let action = action(&fixture, 1, 0, 1.0);
        assert_eq!(
            evaluate_simple_hypothesis_eprocess(
                &fixture, &[action.clone()], &prior(), &[], 1, 1, 0.05,
            ),
            Err(SimpleHypothesisEProcessError::InvalidModelPair)
        );
        assert_eq!(
            evaluate_simple_hypothesis_eprocess(
                &fixture, &[action], &prior(), &[], 1, 2, 1.0,
            ),
            Err(SimpleHypothesisEProcessError::InvalidAlpha)
        );
    }
}
