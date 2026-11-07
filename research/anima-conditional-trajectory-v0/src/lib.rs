//! History-conditioned scalar AR(1) trajectory likelihood with animal random effects.
//!
//! V0 supports one scalar observation per time index, an AR(1) within-session
//! term, and one Gaussian animal-level random intercept shared across sessions.
//! It is a synthetic inference kernel, not a biological locomotion model.

pub mod multivariate;

use std::collections::{BTreeMap, BTreeSet};

const PROBABILITY_TOLERANCE: f64 = 1.0e-10;

pub type ModelPosterior = BTreeMap<String, f64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    SyntheticFixture,
    InSilicoGenerated,
    PreprintMeasured,
    PeerReviewedMeasured,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ar1Mechanism {
    pub model_id: String,
    pub intercept: f64,
    /// Stationary AR(1) coefficient; V0 requires |phi| < 1.
    pub autoregressive_phi: f64,
    /// Explicitly enumerated intervention/action effects. Unknown actions fail.
    pub action_effects: BTreeMap<String, f64>,
    pub innovation_sd: f64,
    /// Animal-level random-intercept standard deviation; zero disables it.
    pub animal_effect_sd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryObservation {
    pub sample_id: String,
    pub animal_id: String,
    pub session_id: String,
    pub time_index: u64,
    pub assigned_action_id: String,
    pub delivered_action_id: String,
    pub assignment_design_valid: bool,
    pub delivery_verified: bool,
    pub delivery_fidelity: f64,
    pub interference_controlled: bool,
    pub carryover_cleared: bool,
    pub evidence_tier: EvidenceTier,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimalTrajectoryHistory {
    pub animal_id: String,
    /// Append-only chronological order. A session may not be reopened after
    /// another session has begun. The animal effect is shared across sessions.
    pub observations: Vec<TrajectoryObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryConfig {
    pub minimum_delivery_fidelity: f64,
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            minimum_delivery_fidelity: 0.95,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalForecast {
    pub model_id: String,
    pub action_id: String,
    pub session_id: String,
    pub time_index: u64,
    pub predictive_mean: f64,
    pub predictive_variance: f64,
    pub animal_effect_posterior_mean: f64,
    pub animal_effect_posterior_variance: f64,
    pub conditioned_observation_count: usize,
    /// FNV-1a drift fingerprint only; not a cryptographic custody receipt.
    pub conditioning_history_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosteriorUpdateReceipt {
    pub observed_sample_id: String,
    pub posterior_after: ModelPosterior,
    pub log_marginal_predictive_density: f64,
    pub conditional_forecasts: BTreeMap<String, ConditionalForecast>,
    pub history_observation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrajectoryError {
    EmptyIdentifier(&'static str),
    InvalidMechanism(String),
    DuplicateMechanism(String),
    UnknownAction {
        model_id: String,
        action_id: String,
    },
    InvalidPosterior,
    ModelSetMismatch,
    InvalidDeliveryThreshold,
    AnimalIdentityMismatch,
    DuplicateSample(String),
    SessionReopened(String),
    OutOfOrderTime {
        session_id: String,
        time_index: u64,
    },
    NonContiguousTime {
        session_id: String,
        expected_time_index: u64,
        actual_time_index: u64,
    },
    TimeIndexOverflow,
    AssignmentDesignInvalid,
    AssignmentDeliveryMismatch,
    DeliveryNotVerified,
    DeliveryFidelityBelowThreshold,
    InvalidDeliveryFidelity,
    InterferenceNotControlled,
    CarryoverNotCleared,
    NonFiniteObservation,
    InvalidPredictiveVariance,
    NumericalFailure,
    NoMechanisms,
}

#[derive(Debug, Clone, Copy)]
struct NormalPosterior {
    mean: f64,
    variance: f64,
}

fn validate_mechanism(model: &Ar1Mechanism) -> Result<(), TrajectoryError> {
    if model.model_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("model_id"));
    }
    if !model.intercept.is_finite()
        || !model.autoregressive_phi.is_finite()
        || model.autoregressive_phi.abs() >= 1.0
        || !model.innovation_sd.is_finite()
        || model.innovation_sd <= 0.0
        || !model.animal_effect_sd.is_finite()
        || model.animal_effect_sd < 0.0
    {
        return Err(TrajectoryError::InvalidMechanism(model.model_id.clone()));
    }
    if model.action_effects.is_empty()
        || model
            .action_effects
            .iter()
            .any(|(id, effect)| id.trim().is_empty() || !effect.is_finite())
    {
        return Err(TrajectoryError::InvalidMechanism(model.model_id.clone()));
    }
    Ok(())
}

fn validate_delivery(
    observation: &TrajectoryObservation,
    minimum_delivery_fidelity: f64,
) -> Result<(), TrajectoryError> {
    if observation.sample_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("sample_id"));
    }
    if observation.animal_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("animal_id"));
    }
    if observation.session_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("session_id"));
    }
    if observation.assigned_action_id.trim().is_empty()
        || observation.delivered_action_id.trim().is_empty()
    {
        return Err(TrajectoryError::EmptyIdentifier("action_id"));
    }
    if !observation.value.is_finite() || !observation.delivery_fidelity.is_finite() {
        return Err(TrajectoryError::NonFiniteObservation);
    }
    if !(0.0..=1.0).contains(&observation.delivery_fidelity) {
        return Err(TrajectoryError::InvalidDeliveryFidelity);
    }
    if !observation.assignment_design_valid {
        return Err(TrajectoryError::AssignmentDesignInvalid);
    }
    if observation.assigned_action_id != observation.delivered_action_id {
        return Err(TrajectoryError::AssignmentDeliveryMismatch);
    }
    if !observation.delivery_verified {
        return Err(TrajectoryError::DeliveryNotVerified);
    }
    if observation.delivery_fidelity < minimum_delivery_fidelity {
        return Err(TrajectoryError::DeliveryFidelityBelowThreshold);
    }
    if !observation.interference_controlled {
        return Err(TrajectoryError::InterferenceNotControlled);
    }
    if !observation.carryover_cleared {
        return Err(TrajectoryError::CarryoverNotCleared);
    }
    Ok(())
}

fn validate_history(
    history: &AnimalTrajectoryHistory,
    config: TrajectoryConfig,
) -> Result<(), TrajectoryError> {
    if history.animal_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("history.animal_id"));
    }
    let mut seen_samples = BTreeSet::new();
    let mut seen_sessions = BTreeSet::new();
    let mut active_session: Option<&str> = None;
    let mut last_time_index = 0_u64;

    for observation in &history.observations {
        validate_delivery(observation, config.minimum_delivery_fidelity)?;
        if observation.animal_id != history.animal_id {
            return Err(TrajectoryError::AnimalIdentityMismatch);
        }
        if !seen_samples.insert(observation.sample_id.as_str()) {
            return Err(TrajectoryError::DuplicateSample(
                observation.sample_id.clone(),
            ));
        }
        if active_session != Some(observation.session_id.as_str()) {
            if seen_sessions.contains(observation.session_id.as_str()) {
                return Err(TrajectoryError::SessionReopened(
                    observation.session_id.clone(),
                ));
            }
            if let Some(previous) = active_session {
                seen_sessions.insert(previous);
            }
            active_session = Some(&observation.session_id);
            last_time_index = observation.time_index;
            seen_sessions.insert(observation.session_id.as_str());
        } else {
            if observation.time_index <= last_time_index {
                return Err(TrajectoryError::OutOfOrderTime {
                    session_id: observation.session_id.clone(),
                    time_index: observation.time_index,
                });
            }
            let expected_time_index = last_time_index
                .checked_add(1)
                .ok_or(TrajectoryError::TimeIndexOverflow)?;
            if observation.time_index != expected_time_index {
                return Err(TrajectoryError::NonContiguousTime {
                    session_id: observation.session_id.clone(),
                    expected_time_index,
                    actual_time_index: observation.time_index,
                });
            }
            last_time_index = observation.time_index;
        }
    }
    Ok(())
}

fn validate_next_identity(
    history: &AnimalTrajectoryHistory,
    session_id: &str,
    time_index: u64,
) -> Result<(), TrajectoryError> {
    if session_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyIdentifier("session_id"));
    }
    if let Some(last) = history.observations.last() {
        if last.session_id == session_id {
            if time_index <= last.time_index {
                return Err(TrajectoryError::OutOfOrderTime {
                    session_id: session_id.to_owned(),
                    time_index,
                });
            }
            let expected_time_index = last
                .time_index
                .checked_add(1)
                .ok_or(TrajectoryError::TimeIndexOverflow)?;
            if time_index != expected_time_index {
                return Err(TrajectoryError::NonContiguousTime {
                    session_id: session_id.to_owned(),
                    expected_time_index,
                    actual_time_index: time_index,
                });
            }
        } else if history
            .observations
            .iter()
            .any(|item| item.session_id == session_id)
        {
            return Err(TrajectoryError::SessionReopened(session_id.to_owned()));
        }
    }
    Ok(())
}

fn action_effect(model: &Ar1Mechanism, action_id: &str) -> Result<f64, TrajectoryError> {
    model
        .action_effects
        .get(action_id)
        .copied()
        .ok_or_else(|| TrajectoryError::UnknownAction {
            model_id: model.model_id.clone(),
            action_id: action_id.to_owned(),
        })
}

/// Integrate the Gaussian animal random intercept using the complete ordered
/// history. The AR state resets at session boundaries; the animal effect does not.
fn animal_effect_posterior(
    model: &Ar1Mechanism,
    history: &AnimalTrajectoryHistory,
) -> Result<NormalPosterior, TrajectoryError> {
    let prior_variance = model.animal_effect_sd.powi(2);
    if prior_variance == 0.0 {
        return Ok(NormalPosterior {
            mean: 0.0,
            variance: 0.0,
        });
    }

    let innovation_variance = model.innovation_sd.powi(2);
    let phi = model.autoregressive_phi;
    let mut precision = 1.0 / prior_variance;
    let mut weighted_residual = 0.0;
    let mut last_by_session: BTreeMap<&str, f64> = BTreeMap::new();

    for observation in &history.observations {
        let action = action_effect(model, &observation.delivered_action_id)?;
        let (base_mean, animal_effect_loading) =
            if let Some(previous) = last_by_session.get(observation.session_id.as_str()) {
                (
                    model.intercept + action + phi * (*previous - model.intercept),
                    1.0 - phi,
                )
            } else {
                (model.intercept + action, 1.0)
            };

        let residual = observation.value - base_mean;
        precision += animal_effect_loading.powi(2) / innovation_variance;
        weighted_residual += animal_effect_loading * residual / innovation_variance;
        if !precision.is_finite() || !weighted_residual.is_finite() {
            return Err(TrajectoryError::NumericalFailure);
        }
        last_by_session.insert(&observation.session_id, observation.value);
    }

    let variance = 1.0 / precision;
    let mean = variance * weighted_residual;
    if !variance.is_finite() || !mean.is_finite() || variance < 0.0 {
        return Err(TrajectoryError::NumericalFailure);
    }
    Ok(NormalPosterior { mean, variance })
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn history_fingerprint(
    model_id: &str,
    history: &AnimalTrajectoryHistory,
    target_session: &str,
    target_time: u64,
    action_id: &str,
) -> String {
    let mut bytes = Vec::new();
    push_bytes(&mut bytes, model_id.as_bytes());
    push_bytes(&mut bytes, history.animal_id.as_bytes());
    push_bytes(&mut bytes, target_session.as_bytes());
    bytes.extend_from_slice(&target_time.to_be_bytes());
    push_bytes(&mut bytes, action_id.as_bytes());
    for observation in &history.observations {
        push_bytes(&mut bytes, observation.sample_id.as_bytes());
        push_bytes(&mut bytes, observation.session_id.as_bytes());
        bytes.extend_from_slice(&observation.time_index.to_be_bytes());
        push_bytes(&mut bytes, observation.assigned_action_id.as_bytes());
        push_bytes(&mut bytes, observation.delivered_action_id.as_bytes());
        bytes.extend_from_slice(&observation.value.to_bits().to_be_bytes());
        bytes.extend_from_slice(&observation.delivery_fidelity.to_bits().to_be_bytes());
        bytes.extend_from_slice(&[
            observation.assignment_design_valid as u8,
            observation.delivery_verified as u8,
            observation.interference_controlled as u8,
            observation.carryover_cleared as u8,
        ]);
        bytes.push(match observation.evidence_tier {
            EvidenceTier::SyntheticFixture => 0,
            EvidenceTier::InSilicoGenerated => 1,
            EvidenceTier::PreprintMeasured => 2,
            EvidenceTier::PeerReviewedMeasured => 3,
        });
    }
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

/// Forecast the next scalar observation conditioned on every accepted prior
/// observation for this animal. A new session resets AR(1) memory but retains
/// the animal-level random-intercept posterior learned from prior sessions.
pub fn forecast_next(
    model: &Ar1Mechanism,
    history: &AnimalTrajectoryHistory,
    session_id: &str,
    time_index: u64,
    action_id: &str,
    config: TrajectoryConfig,
) -> Result<ConditionalForecast, TrajectoryError> {
    validate_mechanism(model)?;
    if !config.minimum_delivery_fidelity.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_delivery_fidelity)
    {
        return Err(TrajectoryError::InvalidDeliveryThreshold);
    }
    validate_history(history, config)?;
    validate_next_identity(history, session_id, time_index)?;
    let action = action_effect(model, action_id)?;
    let animal_effect = animal_effect_posterior(model, history)?;

    let last_in_session = history
        .observations
        .iter()
        .rev()
        .find(|observation| observation.session_id == session_id);
    let (base_mean, animal_effect_loading) = if let Some(previous) = last_in_session {
        (
            model.intercept
                + action
                + model.autoregressive_phi * (previous.value - model.intercept),
            1.0 - model.autoregressive_phi,
        )
    } else {
        (model.intercept + action, 1.0)
    };

    let predictive_mean = base_mean + animal_effect_loading * animal_effect.mean;
    let predictive_variance =
        model.innovation_sd.powi(2) + animal_effect_loading.powi(2) * animal_effect.variance;
    if !predictive_mean.is_finite()
        || !predictive_variance.is_finite()
        || predictive_variance <= 0.0
    {
        return Err(TrajectoryError::InvalidPredictiveVariance);
    }

    Ok(ConditionalForecast {
        model_id: model.model_id.clone(),
        action_id: action_id.to_owned(),
        session_id: session_id.to_owned(),
        time_index,
        predictive_mean,
        predictive_variance,
        animal_effect_posterior_mean: animal_effect.mean,
        animal_effect_posterior_variance: animal_effect.variance,
        conditioned_observation_count: history.observations.len(),
        conditioning_history_fingerprint: history_fingerprint(
            &model.model_id,
            history,
            session_id,
            time_index,
            action_id,
        ),
    })
}

fn gaussian_log_density(value: f64, mean: f64, variance: f64) -> f64 {
    -0.5 * ((2.0 * std::f64::consts::PI * variance).ln() + (value - mean).powi(2) / variance)
}

fn log_sum_exp(values: &[f64]) -> Result<f64, TrajectoryError> {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return Err(TrajectoryError::NumericalFailure);
    }
    let sum: f64 = values.iter().map(|value| (value - maximum).exp()).sum();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(TrajectoryError::NumericalFailure);
    }
    Ok(maximum + sum.ln())
}

fn validate_model_posterior(posterior: &ModelPosterior) -> Result<(), TrajectoryError> {
    if posterior.is_empty()
        || posterior.iter().any(|(id, probability)| {
            id.trim().is_empty() || !probability.is_finite() || !(0.0..=1.0).contains(probability)
        })
        || (posterior.values().sum::<f64>() - 1.0).abs() > PROBABILITY_TOLERANCE
    {
        return Err(TrajectoryError::InvalidPosterior);
    }
    Ok(())
}

/// Add one verified observation and update the mechanism posterior using each
/// model's conditional Gaussian density given the full pre-observation history.
/// The history is mutated only after all models produce valid forecasts.
pub fn update_mechanism_posterior(
    history: &mut AnimalTrajectoryHistory,
    mechanisms: &[Ar1Mechanism],
    posterior_before: &ModelPosterior,
    observation: TrajectoryObservation,
    config: TrajectoryConfig,
) -> Result<PosteriorUpdateReceipt, TrajectoryError> {
    if mechanisms.is_empty() {
        return Err(TrajectoryError::NoMechanisms);
    }
    if !config.minimum_delivery_fidelity.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_delivery_fidelity)
    {
        return Err(TrajectoryError::InvalidDeliveryThreshold);
    }
    validate_history(history, config)?;
    validate_delivery(&observation, config.minimum_delivery_fidelity)?;
    if observation.animal_id != history.animal_id {
        return Err(TrajectoryError::AnimalIdentityMismatch);
    }
    validate_next_identity(history, &observation.session_id, observation.time_index)?;
    if history
        .observations
        .iter()
        .any(|item| item.sample_id == observation.sample_id)
    {
        return Err(TrajectoryError::DuplicateSample(
            observation.sample_id.clone(),
        ));
    }
    validate_model_posterior(posterior_before)?;

    let mut by_id = BTreeMap::new();
    for mechanism in mechanisms {
        validate_mechanism(mechanism)?;
        if by_id
            .insert(mechanism.model_id.clone(), mechanism)
            .is_some()
        {
            return Err(TrajectoryError::DuplicateMechanism(
                mechanism.model_id.clone(),
            ));
        }
    }
    if by_id.keys().ne(posterior_before.keys()) {
        return Err(TrajectoryError::ModelSetMismatch);
    }

    let mut forecasts = BTreeMap::new();
    let mut log_weights = BTreeMap::new();
    for (model_id, prior_probability) in posterior_before {
        let model = by_id[model_id];
        let forecast = forecast_next(
            model,
            history,
            &observation.session_id,
            observation.time_index,
            &observation.delivered_action_id,
            config,
        )?;
        let log_likelihood = gaussian_log_density(
            observation.value,
            forecast.predictive_mean,
            forecast.predictive_variance,
        );
        if !log_likelihood.is_finite() {
            return Err(TrajectoryError::NumericalFailure);
        }
        let log_prior = if *prior_probability == 0.0 {
            f64::NEG_INFINITY
        } else {
            prior_probability.ln()
        };
        log_weights.insert(model_id.clone(), log_prior + log_likelihood);
        forecasts.insert(model_id.clone(), forecast);
    }

    let weights: Vec<f64> = log_weights.values().copied().collect();
    let log_marginal = log_sum_exp(&weights)?;
    let posterior_after: ModelPosterior = log_weights
        .iter()
        .map(|(model_id, log_weight)| (model_id.clone(), (log_weight - log_marginal).exp()))
        .collect();
    validate_model_posterior(&posterior_after)?;

    let observed_sample_id = observation.sample_id.clone();
    history.observations.push(observation);
    Ok(PosteriorUpdateReceipt {
        observed_sample_id,
        posterior_after,
        log_marginal_predictive_density: log_marginal,
        conditional_forecasts: forecasts,
        history_observation_count: history.observations.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, intercept: f64, phi: f64, animal_sd: f64) -> Ar1Mechanism {
        Ar1Mechanism {
            model_id: id.to_owned(),
            intercept,
            autoregressive_phi: phi,
            action_effects: BTreeMap::from([("baseline".to_owned(), 0.0)]),
            innovation_sd: 0.1,
            animal_effect_sd: animal_sd,
        }
    }

    fn observation(
        sample: &str,
        session: &str,
        time_index: u64,
        value: f64,
    ) -> TrajectoryObservation {
        TrajectoryObservation {
            sample_id: sample.to_owned(),
            animal_id: "animal-1".to_owned(),
            session_id: session.to_owned(),
            time_index,
            assigned_action_id: "baseline".to_owned(),
            delivered_action_id: "baseline".to_owned(),
            assignment_design_valid: true,
            delivery_verified: true,
            delivery_fidelity: 1.0,
            interference_controlled: true,
            carryover_cleared: true,
            evidence_tier: EvidenceTier::SyntheticFixture,
            value,
        }
    }

    fn history(records: Vec<TrajectoryObservation>) -> AnimalTrajectoryHistory {
        AnimalTrajectoryHistory {
            animal_id: "animal-1".to_owned(),
            observations: records,
        }
    }

    fn prior(ids: &[&str], probabilities: &[f64]) -> ModelPosterior {
        ids.iter()
            .zip(probabilities)
            .map(|(id, probability)| ((*id).to_owned(), *probability))
            .collect()
    }

    #[test]
    fn forecast_conditions_on_the_previous_observation_and_ar1_state() {
        let model = model("m", 0.0, 0.8, 0.1);
        let history = history(vec![observation("s1", "session-a", 0, 1.0)]);
        let forecast = forecast_next(
            &model,
            &history,
            "session-a",
            1,
            "baseline",
            TrajectoryConfig::default(),
        )
        .unwrap();
        assert!((forecast.predictive_mean - 0.9).abs() < 1.0e-10);
        assert_eq!(forecast.conditioned_observation_count, 1);
    }

    #[test]
    fn repeated_observations_reduce_animal_effect_uncertainty() {
        let model = model("m", 0.0, 0.5, 0.5);
        let once = history(vec![observation("s1", "session-a", 0, 0.4)]);
        let twice = history(vec![
            observation("s1", "session-a", 0, 0.4),
            observation("s2", "session-a", 1, 0.4),
        ]);
        let first = forecast_next(
            &model,
            &once,
            "session-a",
            1,
            "baseline",
            TrajectoryConfig::default(),
        )
        .unwrap();
        let second = forecast_next(
            &model,
            &twice,
            "session-a",
            2,
            "baseline",
            TrajectoryConfig::default(),
        )
        .unwrap();
        assert!(second.animal_effect_posterior_variance < first.animal_effect_posterior_variance);
        assert!(second.predictive_variance < first.predictive_variance);
    }

    #[test]
    fn new_session_resets_ar_state_but_retains_animal_effect_posterior() {
        let model = model("m", 0.0, 0.8, 0.2);
        let history = history(vec![observation("s1", "session-a", 0, 0.8)]);
        let same_session = forecast_next(
            &model,
            &history,
            "session-a",
            1,
            "baseline",
            TrajectoryConfig::default(),
        )
        .unwrap();
        let new_session = forecast_next(
            &model,
            &history,
            "session-b",
            0,
            "baseline",
            TrajectoryConfig::default(),
        )
        .unwrap();
        assert_ne!(same_session.predictive_mean, new_session.predictive_mean);
        assert_eq!(
            same_session.animal_effect_posterior_mean,
            new_session.animal_effect_posterior_mean
        );
    }

    #[test]
    fn model_posterior_uses_conditional_gaussian_predictive_density() {
        let mechanisms = vec![model("low", 0.0, 0.0, 0.0), model("high", 1.0, 0.0, 0.0)];
        let mut history = history(Vec::new());
        let receipt = update_mechanism_posterior(
            &mut history,
            &mechanisms,
            &prior(&["low", "high"], &[0.5, 0.5]),
            observation("sample-1", "session-a", 0, 1.0),
            TrajectoryConfig::default(),
        )
        .unwrap();
        assert!(receipt.posterior_after["high"] > 0.999);
        assert_eq!(receipt.history_observation_count, 1);
        assert_eq!(history.observations.len(), 1);
    }

    #[test]
    fn assigned_and_delivered_action_must_match() {
        let mut history = history(Vec::new());
        let mut record = observation("sample-1", "session-a", 0, 0.0);
        record.delivered_action_id = "different-protocol".to_owned();
        let result = update_mechanism_posterior(
            &mut history,
            &[model("m", 0.0, 0.0, 0.1)],
            &prior(&["m"], &[1.0]),
            record,
            TrajectoryConfig::default(),
        );
        assert_eq!(result, Err(TrajectoryError::AssignmentDeliveryMismatch));
        assert!(history.observations.is_empty());
    }

    #[test]
    fn duplicate_samples_and_non_monotonic_times_are_rejected() {
        let existing = observation("sample-1", "session-a", 2, 0.0);
        let duplicate = observation("sample-1", "session-a", 3, 0.0);
        let duplicate_history = history(vec![existing.clone()]);
        assert_eq!(
            forecast_next(
                &model("m", 0.0, 0.0, 0.1),
                &duplicate_history,
                "session-a",
                3,
                "baseline",
                TrajectoryConfig::default()
            )
            .unwrap()
            .conditioned_observation_count,
            1
        );
        let mut update_history = history(vec![existing]);
        let result = update_mechanism_posterior(
            &mut update_history,
            &[model("m", 0.0, 0.0, 0.1)],
            &prior(&["m"], &[1.0]),
            duplicate,
            TrajectoryConfig::default(),
        );
        assert_eq!(
            result,
            Err(TrajectoryError::DuplicateSample("sample-1".to_owned()))
        );

        let out_of_order = forecast_next(
            &model("m", 0.0, 0.0, 0.1),
            &update_history,
            "session-a",
            1,
            "baseline",
            TrajectoryConfig::default(),
        );
        assert!(matches!(
            out_of_order,
            Err(TrajectoryError::OutOfOrderTime { .. })
        ));
    }

    #[test]
    fn irregular_within_session_time_indices_are_rejected_instead_of_misapplied_as_one_ar_step() {
        let existing = history(vec![observation("sample-1", "session-a", 0, 0.0)]);
        assert_eq!(
            forecast_next(
                &model("m", 0.0, 0.5, 0.1),
                &existing,
                "session-a",
                2,
                "baseline",
                TrajectoryConfig::default()
            ),
            Err(TrajectoryError::NonContiguousTime {
                session_id: "session-a".to_owned(),
                expected_time_index: 1,
                actual_time_index: 2,
            })
        );
    }

    #[test]
    fn unknown_actions_are_not_silently_mapped_to_zero() {
        let model = model("m", 0.0, 0.0, 0.1);
        let result = forecast_next(
            &model,
            &history(Vec::new()),
            "session-a",
            0,
            "unregistered-action",
            TrajectoryConfig::default(),
        );
        assert_eq!(
            result,
            Err(TrajectoryError::UnknownAction {
                model_id: "m".to_owned(),
                action_id: "unregistered-action".to_owned(),
            })
        );
    }

    #[test]
    fn session_cannot_be_reopened_after_a_later_session() {
        let invalid = history(vec![
            observation("s1", "session-a", 0, 0.0),
            observation("s2", "session-b", 0, 0.0),
            observation("s3", "session-a", 1, 0.0),
        ]);
        assert_eq!(
            forecast_next(
                &model("m", 0.0, 0.0, 0.1),
                &invalid,
                "session-b",
                1,
                "baseline",
                TrajectoryConfig::default()
            ),
            Err(TrajectoryError::SessionReopened("session-a".to_owned()))
        );
    }

    #[test]
    fn delivery_interference_and_carryover_gates_fail_closed() {
        for mutate in 0..3 {
            let mut record = observation("sample", "session", 0, 0.0);
            match mutate {
                0 => record.delivery_verified = false,
                1 => record.interference_controlled = false,
                _ => record.carryover_cleared = false,
            }
            let mut history = history(Vec::new());
            let result = update_mechanism_posterior(
                &mut history,
                &[model("m", 0.0, 0.0, 0.1)],
                &prior(&["m"], &[1.0]),
                record,
                TrajectoryConfig::default(),
            );
            assert!(result.is_err());
            assert!(history.observations.is_empty());
        }
    }
}
