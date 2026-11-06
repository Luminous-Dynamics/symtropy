//! Multichannel linear-Gaussian latent-phase trajectory kernel.
//!
//! One latent AR(1) coordination state is shared across observed channels;
//! each channel also has its own static animal random intercept. Session
//! boundaries reset latent phase state while retaining animal effects.
//!
//! Exactness is limited to this declared Gaussian model family. Missing
//! channel values are omitted from the likelihood under an ignorable-missingness
//! assumption; this module does not model informative missingness.

use std::collections::{BTreeMap, BTreeSet};

const PROBABILITY_TOLERANCE: f64 = 1.0e-10;
const COVARIANCE_TOLERANCE: f64 = 1.0e-10;

pub type MechanismPosterior = BTreeMap<String, f64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    SyntheticFixture,
    InSilicoGenerated,
    PreprintMeasured,
    PeerReviewedMeasured,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputChannel {
    pub channel_id: String,
    pub intercept: f64,
    /// Loading of the shared latent coordination state.
    pub phase_loading: f64,
    /// Explicit action effects. Unknown actions fail closed.
    pub action_effects: BTreeMap<String, f64>,
    pub observation_sd: f64,
    /// Independent channel-specific animal random-intercept standard deviation.
    pub animal_effect_sd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultivariateMechanism {
    pub model_id: String,
    /// Shared latent coordination state follows a stationary AR(1) process.
    pub phase_phi: f64,
    pub phase_innovation_sd: f64,
    pub channels: Vec<OutputChannel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultichannelObservation {
    pub sample_id: String,
    pub animal_id: String,
    pub session_id: String,
    /// Session-local integer time index. V0 requires contiguous indices.
    pub time_index: u64,
    pub assigned_action_id: String,
    pub delivered_action_id: String,
    pub assignment_design_valid: bool,
    pub delivery_verified: bool,
    pub delivery_fidelity: f64,
    pub interference_controlled: bool,
    pub carryover_cleared: bool,
    pub evidence_tier: EvidenceTier,
    /// Any subset of channels may be observed at a time point.
    pub values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultichannelHistory {
    pub animal_id: String,
    pub observations: Vec<MultichannelObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultivariateConfig {
    pub minimum_delivery_fidelity: f64,
}

impl Default for MultivariateConfig {
    fn default() -> Self {
        Self {
            minimum_delivery_fidelity: 0.95,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultivariateForecast {
    pub model_id: String,
    pub action_id: String,
    pub session_id: String,
    pub time_index: u64,
    /// Sorted channel IDs define the row/column order of covariance.
    pub channel_ids: Vec<String>,
    pub predictive_means: Vec<f64>,
    /// Full predictive covariance, including shared latent-state covariance.
    pub predictive_covariance: Vec<Vec<f64>>,
    pub latent_phase_mean: f64,
    pub latent_phase_variance: f64,
    pub conditioned_observation_count: usize,
    /// FNV-1a drift fingerprint only; not a cryptographic receipt.
    pub conditioning_history_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultivariateUpdateReceipt {
    pub observed_sample_id: String,
    pub posterior_after: MechanismPosterior,
    pub conditional_log_likelihoods: BTreeMap<String, f64>,
    pub forecasts_before_observation: BTreeMap<String, MultivariateForecast>,
    pub history_observation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultivariateError {
    EmptyIdentifier(&'static str),
    EmptyChannelSet,
    DuplicateChannel(String),
    DuplicateMechanism(String),
    InvalidMechanism(String),
    InvalidActionEffect {
        model_id: String,
        channel_id: String,
        action_id: String,
    },
    UnknownChannel {
        model_id: String,
        channel_id: String,
    },
    InvalidPosterior,
    ModelSetMismatch,
    InvalidDeliveryThreshold,
    InvalidDeliveryFidelity,
    AnimalIdentityMismatch,
    DuplicateSample(String),
    SessionReopened(String),
    SessionStartTimeNotZero {
        session_id: String,
        actual_time_index: u64,
    },
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
    EmptyObservationValues,
    AssignmentDesignInvalid,
    AssignmentDeliveryMismatch,
    DeliveryNotVerified,
    DeliveryFidelityBelowThreshold,
    InterferenceNotControlled,
    CarryoverNotCleared,
    NonFiniteObservation,
    InvalidPredictiveCovariance,
    NumericalFailure,
    NoMechanisms,
}

#[derive(Debug, Clone)]
struct GaussianState {
    /// State vector is [shared latent phase, animal_effect[channel_0], ...].
    mean: Vec<f64>,
    covariance: Vec<Vec<f64>>,
    active_session: Option<String>,
    last_time_index: Option<u64>,
}

impl GaussianState {
    fn initial(model: &MultivariateMechanism) -> Self {
        let dimension = model.channels.len() + 1;
        let phase_variance = model.phase_innovation_sd.powi(2) / (1.0 - model.phase_phi.powi(2));
        let mut covariance = vec![vec![0.0; dimension]; dimension];
        covariance[0][0] = phase_variance;
        for (index, channel) in model.channels.iter().enumerate() {
            covariance[index + 1][index + 1] = channel.animal_effect_sd.powi(2);
        }
        Self {
            mean: vec![0.0; dimension],
            covariance,
            active_session: None,
            last_time_index: None,
        }
    }

    fn reset_session(&mut self, model: &MultivariateMechanism, session_id: &str) {
        let phase_variance = model.phase_innovation_sd.powi(2) / (1.0 - model.phase_phi.powi(2));
        self.mean[0] = 0.0;
        for index in 1..self.mean.len() {
            self.covariance[0][index] = 0.0;
            self.covariance[index][0] = 0.0;
        }
        self.covariance[0][0] = phase_variance;
        self.active_session = Some(session_id.to_owned());
        self.last_time_index = None;
    }

    fn propagate(&mut self, model: &MultivariateMechanism) -> Result<(), MultivariateError> {
        let phi = model.phase_phi;
        self.mean[0] *= phi;
        self.covariance[0][0] =
            phi.powi(2) * self.covariance[0][0] + model.phase_innovation_sd.powi(2);
        for index in 1..self.mean.len() {
            self.covariance[0][index] *= phi;
            self.covariance[index][0] = self.covariance[0][index];
        }
        if !self.mean[0].is_finite()
            || !self
                .covariance
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        {
            return Err(MultivariateError::NumericalFailure);
        }
        Ok(())
    }

    fn observe_channel(
        &mut self,
        model: &MultivariateMechanism,
        channel_index: usize,
        action_id: &str,
        observed_value: f64,
    ) -> Result<f64, MultivariateError> {
        let channel = &model.channels[channel_index];
        let action_effect = channel
            .action_effects
            .get(action_id)
            .copied()
            .ok_or_else(|| MultivariateError::InvalidActionEffect {
                model_id: model.model_id.clone(),
                channel_id: channel.channel_id.clone(),
                action_id: action_id.to_owned(),
            })?;

        let mut loading = vec![0.0; self.mean.len()];
        loading[0] = channel.phase_loading;
        loading[channel_index + 1] = 1.0;

        let mut covariance_loading = vec![0.0; self.mean.len()];
        for (covariance_loading_row, covariance_row) in
            covariance_loading.iter_mut().zip(&self.covariance)
        {
            *covariance_loading_row = covariance_row
                .iter()
                .zip(&loading)
                .map(|(covariance, coefficient)| covariance * coefficient)
                .sum();
        }

        let predicted_state = loading
            .iter()
            .zip(&self.mean)
            .map(|(coefficient, value)| coefficient * value)
            .sum::<f64>();
        let predicted_mean = channel.intercept + action_effect + predicted_state;
        let innovation_variance = channel.observation_sd.powi(2)
            + loading
                .iter()
                .zip(&covariance_loading)
                .map(|(coefficient, covariance)| coefficient * covariance)
                .sum::<f64>();
        if !predicted_mean.is_finite()
            || !innovation_variance.is_finite()
            || innovation_variance <= 0.0
        {
            return Err(MultivariateError::InvalidPredictiveCovariance);
        }

        let residual = observed_value - predicted_mean;
        let log_density = -0.5
            * ((2.0 * std::f64::consts::PI * innovation_variance).ln()
                + residual.powi(2) / innovation_variance);
        if !log_density.is_finite() {
            return Err(MultivariateError::NumericalFailure);
        }

        let old_covariance = self.covariance.clone();
        for (mean, covariance_loading_row) in self.mean.iter_mut().zip(&covariance_loading) {
            *mean += covariance_loading_row * residual / innovation_variance;
        }
        for row in 0..self.mean.len() {
            for column in 0..self.mean.len() {
                self.covariance[row][column] = old_covariance[row][column]
                    - covariance_loading[row] * covariance_loading[column] / innovation_variance;
            }
        }
        for row in 0..self.mean.len() {
            for column in 0..row {
                let symmetric = 0.5 * (self.covariance[row][column] + self.covariance[column][row]);
                self.covariance[row][column] = symmetric;
                self.covariance[column][row] = symmetric;
            }
        }
        if !self.mean.iter().all(|value| value.is_finite())
            || !self
                .covariance
                .iter()
                .flatten()
                .all(|value| value.is_finite())
            || self
                .covariance
                .iter()
                .enumerate()
                .any(|(index, row)| row[index] < -COVARIANCE_TOLERANCE)
        {
            return Err(MultivariateError::NumericalFailure);
        }
        Ok(log_density)
    }
}

fn validate_model(model: &MultivariateMechanism) -> Result<(), MultivariateError> {
    if model.model_id.trim().is_empty() {
        return Err(MultivariateError::EmptyIdentifier("model_id"));
    }
    if model.channels.len() < 2 {
        return Err(MultivariateError::EmptyChannelSet);
    }
    if !model.phase_phi.is_finite()
        || model.phase_phi.abs() >= 1.0
        || !model.phase_innovation_sd.is_finite()
        || model.phase_innovation_sd <= 0.0
    {
        return Err(MultivariateError::InvalidMechanism(model.model_id.clone()));
    }
    let mut channel_ids = BTreeSet::new();
    for channel in &model.channels {
        if channel.channel_id.trim().is_empty() {
            return Err(MultivariateError::EmptyIdentifier("channel_id"));
        }
        if !channel_ids.insert(channel.channel_id.clone()) {
            return Err(MultivariateError::DuplicateChannel(
                channel.channel_id.clone(),
            ));
        }
        if !channel.intercept.is_finite()
            || !channel.phase_loading.is_finite()
            || !channel.observation_sd.is_finite()
            || channel.observation_sd <= 0.0
            || !channel.animal_effect_sd.is_finite()
            || channel.animal_effect_sd < 0.0
            || channel.action_effects.is_empty()
            || channel
                .action_effects
                .iter()
                .any(|(id, effect)| id.trim().is_empty() || !effect.is_finite())
        {
            return Err(MultivariateError::InvalidMechanism(model.model_id.clone()));
        }
    }
    Ok(())
}

fn validate_observation(
    observation: &MultichannelObservation,
    minimum_delivery_fidelity: f64,
) -> Result<(), MultivariateError> {
    for (field, value) in [
        ("sample_id", observation.sample_id.as_str()),
        ("animal_id", observation.animal_id.as_str()),
        ("session_id", observation.session_id.as_str()),
        (
            "assigned_action_id",
            observation.assigned_action_id.as_str(),
        ),
        (
            "delivered_action_id",
            observation.delivered_action_id.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(MultivariateError::EmptyIdentifier(field));
        }
    }
    if observation.values.is_empty() {
        return Err(MultivariateError::EmptyObservationValues);
    }
    if observation.values.values().any(|value| !value.is_finite())
        || !observation.delivery_fidelity.is_finite()
    {
        return Err(MultivariateError::NonFiniteObservation);
    }
    if !(0.0..=1.0).contains(&observation.delivery_fidelity) {
        return Err(MultivariateError::InvalidDeliveryFidelity);
    }
    if !observation.assignment_design_valid {
        return Err(MultivariateError::AssignmentDesignInvalid);
    }
    if observation.assigned_action_id != observation.delivered_action_id {
        return Err(MultivariateError::AssignmentDeliveryMismatch);
    }
    if !observation.delivery_verified {
        return Err(MultivariateError::DeliveryNotVerified);
    }
    if observation.delivery_fidelity < minimum_delivery_fidelity {
        return Err(MultivariateError::DeliveryFidelityBelowThreshold);
    }
    if !observation.interference_controlled {
        return Err(MultivariateError::InterferenceNotControlled);
    }
    if !observation.carryover_cleared {
        return Err(MultivariateError::CarryoverNotCleared);
    }
    Ok(())
}

fn channel_index(model: &MultivariateMechanism) -> BTreeMap<String, usize> {
    model
        .channels
        .iter()
        .enumerate()
        .map(|(index, channel)| (channel.channel_id.clone(), index))
        .collect()
}

fn process_session_observation(
    state: &mut GaussianState,
    model: &MultivariateMechanism,
    observation: &MultichannelObservation,
    indices: &BTreeMap<String, usize>,
) -> Result<f64, MultivariateError> {
    if state.active_session.as_deref() != Some(observation.session_id.as_str()) {
        state.reset_session(model, &observation.session_id);
        if observation.time_index != 0 {
            return Err(MultivariateError::SessionStartTimeNotZero {
                session_id: observation.session_id.clone(),
                actual_time_index: observation.time_index,
            });
        }
    } else {
        let previous = state
            .last_time_index
            .ok_or(MultivariateError::NumericalFailure)?;
        if observation.time_index <= previous {
            return Err(MultivariateError::OutOfOrderTime {
                session_id: observation.session_id.clone(),
                time_index: observation.time_index,
            });
        }
        let expected = previous
            .checked_add(1)
            .ok_or(MultivariateError::TimeIndexOverflow)?;
        if observation.time_index != expected {
            return Err(MultivariateError::NonContiguousTime {
                session_id: observation.session_id.clone(),
                expected_time_index: expected,
                actual_time_index: observation.time_index,
            });
        }
        state.propagate(model)?;
    }

    let mut log_likelihood = 0.0;
    for (channel_id, value) in &observation.values {
        let index = *indices
            .get(channel_id)
            .ok_or_else(|| MultivariateError::UnknownChannel {
                model_id: model.model_id.clone(),
                channel_id: channel_id.clone(),
            })?;
        log_likelihood +=
            state.observe_channel(model, index, &observation.delivered_action_id, *value)?;
        if !log_likelihood.is_finite() {
            return Err(MultivariateError::NumericalFailure);
        }
    }
    state.last_time_index = Some(observation.time_index);
    Ok(log_likelihood)
}

fn validate_history(
    history: &MultichannelHistory,
    config: MultivariateConfig,
) -> Result<(), MultivariateError> {
    if history.animal_id.trim().is_empty() {
        return Err(MultivariateError::EmptyIdentifier("history.animal_id"));
    }
    let mut sample_ids = BTreeSet::new();
    let mut completed_sessions = BTreeSet::new();
    let mut active_session: Option<String> = None;
    let mut last_time_index = None;

    for observation in &history.observations {
        validate_observation(observation, config.minimum_delivery_fidelity)?;
        if observation.animal_id != history.animal_id {
            return Err(MultivariateError::AnimalIdentityMismatch);
        }
        if !sample_ids.insert(observation.sample_id.as_str()) {
            return Err(MultivariateError::DuplicateSample(
                observation.sample_id.clone(),
            ));
        }
        if active_session.as_deref() != Some(observation.session_id.as_str()) {
            if completed_sessions.contains(&observation.session_id) {
                return Err(MultivariateError::SessionReopened(
                    observation.session_id.clone(),
                ));
            }
            if let Some(previous) = active_session.replace(observation.session_id.clone()) {
                completed_sessions.insert(previous);
            }
            if observation.time_index != 0 {
                return Err(MultivariateError::SessionStartTimeNotZero {
                    session_id: observation.session_id.clone(),
                    actual_time_index: observation.time_index,
                });
            }
            last_time_index = Some(0_u64);
        } else {
            let previous = last_time_index.ok_or(MultivariateError::NumericalFailure)?;
            if observation.time_index <= previous {
                return Err(MultivariateError::OutOfOrderTime {
                    session_id: observation.session_id.clone(),
                    time_index: observation.time_index,
                });
            }
            let expected = previous
                .checked_add(1)
                .ok_or(MultivariateError::TimeIndexOverflow)?;
            if observation.time_index != expected {
                return Err(MultivariateError::NonContiguousTime {
                    session_id: observation.session_id.clone(),
                    expected_time_index: expected,
                    actual_time_index: observation.time_index,
                });
            }
            last_time_index = Some(observation.time_index);
        }
    }
    Ok(())
}

fn validate_next_identity(
    history: &MultichannelHistory,
    session_id: &str,
    time_index: u64,
) -> Result<(), MultivariateError> {
    if session_id.trim().is_empty() {
        return Err(MultivariateError::EmptyIdentifier("session_id"));
    }
    match history.observations.last() {
        None if time_index != 0 => Err(MultivariateError::SessionStartTimeNotZero {
            session_id: session_id.to_owned(),
            actual_time_index: time_index,
        }),
        None => Ok(()),
        Some(last) if last.session_id == session_id => {
            if time_index <= last.time_index {
                return Err(MultivariateError::OutOfOrderTime {
                    session_id: session_id.to_owned(),
                    time_index,
                });
            }
            let expected = last
                .time_index
                .checked_add(1)
                .ok_or(MultivariateError::TimeIndexOverflow)?;
            if time_index != expected {
                return Err(MultivariateError::NonContiguousTime {
                    session_id: session_id.to_owned(),
                    expected_time_index: expected,
                    actual_time_index: time_index,
                });
            }
            Ok(())
        }
        Some(_)
            if history
                .observations
                .iter()
                .any(|item| item.session_id == session_id) =>
        {
            Err(MultivariateError::SessionReopened(session_id.to_owned()))
        }
        Some(_) if time_index != 0 => Err(MultivariateError::SessionStartTimeNotZero {
            session_id: session_id.to_owned(),
            actual_time_index: time_index,
        }),
        Some(_) => Ok(()),
    }
}

fn filter_history(
    model: &MultivariateMechanism,
    history: &MultichannelHistory,
    config: MultivariateConfig,
) -> Result<GaussianState, MultivariateError> {
    validate_model(model)?;
    validate_history(history, config)?;
    let indices = channel_index(model);
    for observation in &history.observations {
        for channel_id in observation.values.keys() {
            if !indices.contains_key(channel_id) {
                return Err(MultivariateError::UnknownChannel {
                    model_id: model.model_id.clone(),
                    channel_id: channel_id.clone(),
                });
            }
        }
    }
    let mut state = GaussianState::initial(model);
    for observation in &history.observations {
        process_session_observation(&mut state, model, observation, &indices)?;
    }
    Ok(state)
}

fn forecast_from_state(
    model: &MultivariateMechanism,
    state: &GaussianState,
    session_id: &str,
    time_index: u64,
    action_id: &str,
    conditioned_observation_count: usize,
    fingerprint: String,
) -> Result<MultivariateForecast, MultivariateError> {
    let indices = channel_index(model);
    let mut channel_ids: Vec<String> = indices.keys().cloned().collect();
    channel_ids.sort();
    let dimension = state.mean.len();

    let mut loadings: Vec<Vec<f64>> = Vec::with_capacity(channel_ids.len());
    let mut predictive_means = Vec::with_capacity(channel_ids.len());
    for channel_id in &channel_ids {
        let index = indices[channel_id];
        let channel = &model.channels[index];
        let action_effect = channel
            .action_effects
            .get(action_id)
            .copied()
            .ok_or_else(|| MultivariateError::InvalidActionEffect {
                model_id: model.model_id.clone(),
                channel_id: channel.channel_id.clone(),
                action_id: action_id.to_owned(),
            })?;
        let mut loading = vec![0.0; dimension];
        loading[0] = channel.phase_loading;
        loading[index + 1] = 1.0;
        let state_mean = loading
            .iter()
            .zip(&state.mean)
            .map(|(coefficient, value)| coefficient * value)
            .sum::<f64>();
        predictive_means.push(channel.intercept + action_effect + state_mean);
        loadings.push(loading);
    }

    let mut predictive_covariance = vec![vec![0.0; channel_ids.len()]; channel_ids.len()];
    for row in 0..channel_ids.len() {
        for column in 0..channel_ids.len() {
            let mut covariance = 0.0;
            for left in 0..dimension {
                for right in 0..dimension {
                    covariance += loadings[row][left]
                        * state.covariance[left][right]
                        * loadings[column][right];
                }
            }
            if row == column {
                let index = indices[&channel_ids[row]];
                covariance += model.channels[index].observation_sd.powi(2);
            }
            predictive_covariance[row][column] = covariance;
        }
    }
    if predictive_means.iter().any(|value| !value.is_finite())
        || predictive_covariance
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        || predictive_covariance
            .iter()
            .enumerate()
            .any(|(index, row)| row[index] <= 0.0)
    {
        return Err(MultivariateError::InvalidPredictiveCovariance);
    }

    Ok(MultivariateForecast {
        model_id: model.model_id.clone(),
        action_id: action_id.to_owned(),
        session_id: session_id.to_owned(),
        time_index,
        channel_ids,
        predictive_means,
        predictive_covariance,
        latent_phase_mean: state.mean[0],
        latent_phase_variance: state.covariance[0][0],
        conditioned_observation_count,
        conditioning_history_fingerprint: fingerprint,
    })
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn history_fingerprint(
    model_id: &str,
    history: &MultichannelHistory,
    session_id: &str,
    time_index: u64,
    action_id: &str,
) -> String {
    let mut bytes = Vec::new();
    push_bytes(&mut bytes, model_id.as_bytes());
    push_bytes(&mut bytes, history.animal_id.as_bytes());
    push_bytes(&mut bytes, session_id.as_bytes());
    bytes.extend_from_slice(&time_index.to_be_bytes());
    push_bytes(&mut bytes, action_id.as_bytes());
    for observation in &history.observations {
        push_bytes(&mut bytes, observation.sample_id.as_bytes());
        push_bytes(&mut bytes, observation.session_id.as_bytes());
        bytes.extend_from_slice(&observation.time_index.to_be_bytes());
        push_bytes(&mut bytes, observation.assigned_action_id.as_bytes());
        push_bytes(&mut bytes, observation.delivered_action_id.as_bytes());
        bytes.extend_from_slice(&observation.delivery_fidelity.to_bits().to_be_bytes());
        bytes.extend_from_slice(&observation.evidence_tier_as_byte().to_be_bytes());
        for (channel_id, value) in &observation.values {
            push_bytes(&mut bytes, channel_id.as_bytes());
            bytes.extend_from_slice(&value.to_bits().to_be_bytes());
        }
    }
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

impl MultichannelObservation {
    fn evidence_tier_as_byte(&self) -> u8 {
        match self.evidence_tier {
            EvidenceTier::SyntheticFixture => 0,
            EvidenceTier::InSilicoGenerated => 1,
            EvidenceTier::PreprintMeasured => 2,
            EvidenceTier::PeerReviewedMeasured => 3,
        }
    }
}

/// Forecast all declared channels given the complete prior history of one
/// animal. Missing channels in prior observations are marginalized by omission
/// under the declared ignorable-missingness assumption.
pub fn forecast_next(
    model: &MultivariateMechanism,
    history: &MultichannelHistory,
    session_id: &str,
    time_index: u64,
    action_id: &str,
    config: MultivariateConfig,
) -> Result<MultivariateForecast, MultivariateError> {
    validate_model(model)?;
    validate_config(config)?;
    validate_next_identity(history, session_id, time_index)?;
    let mut state = filter_history(model, history, config)?;
    if state.active_session.as_deref() == Some(session_id) {
        state.propagate(model)?;
    } else {
        state.reset_session(model, session_id);
    }
    let fingerprint =
        history_fingerprint(&model.model_id, history, session_id, time_index, action_id);
    forecast_from_state(
        model,
        &state,
        session_id,
        time_index,
        action_id,
        history.observations.len(),
        fingerprint,
    )
}

fn validate_config(config: MultivariateConfig) -> Result<(), MultivariateError> {
    if !config.minimum_delivery_fidelity.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_delivery_fidelity)
    {
        return Err(MultivariateError::InvalidDeliveryThreshold);
    }
    Ok(())
}

fn validate_posterior(posterior: &MechanismPosterior) -> Result<(), MultivariateError> {
    if posterior.is_empty()
        || posterior.iter().any(|(id, probability)| {
            id.trim().is_empty() || !probability.is_finite() || !(0.0..=1.0).contains(probability)
        })
        || (posterior.values().sum::<f64>() - 1.0).abs() > PROBABILITY_TOLERANCE
    {
        return Err(MultivariateError::InvalidPosterior);
    }
    Ok(())
}

/// Add one multichannel sample and update model probabilities from its joint
/// conditional predictive density. Channel updates are sequential Kalman
/// innovations in canonical channel-ID order, exactly factorizing the joint
/// Gaussian likelihood. History is mutated only after every model succeeds.
pub fn update_mechanism_posterior(
    history: &mut MultichannelHistory,
    mechanisms: &[MultivariateMechanism],
    posterior_before: &MechanismPosterior,
    observation: MultichannelObservation,
    config: MultivariateConfig,
) -> Result<MultivariateUpdateReceipt, MultivariateError> {
    if mechanisms.is_empty() {
        return Err(MultivariateError::NoMechanisms);
    }
    validate_config(config)?;
    validate_history(history, config)?;
    validate_observation(&observation, config.minimum_delivery_fidelity)?;
    if observation.animal_id != history.animal_id {
        return Err(MultivariateError::AnimalIdentityMismatch);
    }
    validate_next_identity(history, &observation.session_id, observation.time_index)?;
    if history
        .observations
        .iter()
        .any(|item| item.sample_id == observation.sample_id)
    {
        return Err(MultivariateError::DuplicateSample(
            observation.sample_id.clone(),
        ));
    }
    validate_posterior(posterior_before)?;

    let mut by_id = BTreeMap::new();
    for model in mechanisms {
        validate_model(model)?;
        if by_id.insert(model.model_id.clone(), model).is_some() {
            return Err(MultivariateError::DuplicateMechanism(
                model.model_id.clone(),
            ));
        }
    }
    if by_id.keys().ne(posterior_before.keys()) {
        return Err(MultivariateError::ModelSetMismatch);
    }

    let mut forecasts_before_observation = BTreeMap::new();
    let mut conditional_log_likelihoods = BTreeMap::new();
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
        for channel_id in observation.values.keys() {
            if !forecast.channel_ids.contains(channel_id) {
                return Err(MultivariateError::UnknownChannel {
                    model_id: model_id.clone(),
                    channel_id: channel_id.clone(),
                });
            }
        }

        let log_likelihood =
            process_session_observation_score(model, history, &observation, config)?;
        conditional_log_likelihoods.insert(model_id.clone(), log_likelihood);
        forecasts_before_observation.insert(model_id.clone(), forecast);

        let log_prior = if *prior_probability == 0.0 {
            f64::NEG_INFINITY
        } else {
            prior_probability.ln()
        };
        log_weights.insert(model_id.clone(), log_prior + log_likelihood);
    }

    let weights: Vec<f64> = log_weights.values().copied().collect();
    let log_marginal = log_sum_exp(&weights)?;
    let posterior_after: MechanismPosterior = log_weights
        .iter()
        .map(|(id, weight)| (id.clone(), (weight - log_marginal).exp()))
        .collect();
    validate_posterior(&posterior_after)?;

    let observed_sample_id = observation.sample_id.clone();
    history.observations.push(observation);
    Ok(MultivariateUpdateReceipt {
        observed_sample_id,
        posterior_after,
        conditional_log_likelihoods,
        forecasts_before_observation,
        history_observation_count: history.observations.len(),
    })
}

fn process_session_observation_score(
    model: &MultivariateMechanism,
    history: &MultichannelHistory,
    observation: &MultichannelObservation,
    config: MultivariateConfig,
) -> Result<f64, MultivariateError> {
    let mut state = filter_history(model, history, config)?;
    let indices = channel_index(model);
    process_session_observation(&mut state, model, observation, &indices)
}

fn log_sum_exp(values: &[f64]) -> Result<f64, MultivariateError> {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return Err(MultivariateError::NumericalFailure);
    }
    let sum: f64 = values.iter().map(|value| (value - maximum).exp()).sum();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(MultivariateError::NumericalFailure);
    }
    Ok(maximum + sum.ln())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(id: &str, phase_loading: f64, animal_sd: f64) -> OutputChannel {
        OutputChannel {
            channel_id: id.to_owned(),
            intercept: if id == "joint-angle" { 0.1 } else { 0.2 },
            phase_loading,
            action_effects: BTreeMap::from([
                ("baseline".to_owned(), 0.0),
                ("stim".to_owned(), 0.5),
            ]),
            observation_sd: 0.1,
            animal_effect_sd: animal_sd,
        }
    }

    fn model(id: &str, phase_phi: f64, loading: f64) -> MultivariateMechanism {
        MultivariateMechanism {
            model_id: id.to_owned(),
            phase_phi,
            phase_innovation_sd: 0.2,
            channels: vec![
                channel("joint-angle", loading, 0.2),
                channel("contact-force", 2.0 * loading, 0.3),
            ],
        }
    }

    fn observation(
        sample: &str,
        session: &str,
        time_index: u64,
        values: &[(&str, f64)],
    ) -> MultichannelObservation {
        MultichannelObservation {
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
            values: values
                .iter()
                .map(|(id, value)| ((*id).to_owned(), *value))
                .collect(),
        }
    }

    fn history(records: Vec<MultichannelObservation>) -> MultichannelHistory {
        MultichannelHistory {
            animal_id: "animal-1".to_owned(),
            observations: records,
        }
    }

    fn prior(ids: &[&str], probabilities: &[f64]) -> MechanismPosterior {
        ids.iter()
            .zip(probabilities)
            .map(|(id, probability)| ((*id).to_owned(), *probability))
            .collect()
    }

    #[test]
    fn forecast_exposes_cross_channel_covariance_from_shared_latent_phase() {
        let forecast = forecast_next(
            &model("m", 0.8, 1.0),
            &history(Vec::new()),
            "session-a",
            0,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        assert_eq!(
            forecast.channel_ids,
            vec!["contact-force".to_owned(), "joint-angle".to_owned()]
        );
        assert!(forecast.predictive_covariance[0][1] > 0.0);
        assert_eq!(
            forecast.predictive_covariance[0][1],
            forecast.predictive_covariance[1][0]
        );
    }

    #[test]
    fn observing_one_channel_updates_the_shared_latent_state_for_another() {
        let model = model("m", 0.8, 1.0);
        let empty = history(Vec::new());
        let before = forecast_next(
            &model,
            &empty,
            "session-a",
            0,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        let mut after_history = empty;
        after_history
            .observations
            .push(observation("s1", "session-a", 0, &[("joint-angle", 1.0)]));
        let after = forecast_next(
            &model,
            &after_history,
            "session-a",
            1,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        let force_index = after
            .channel_ids
            .iter()
            .position(|id| id == "contact-force")
            .unwrap();
        assert_ne!(
            before.predictive_means[force_index],
            after.predictive_means[force_index]
        );
        assert!(after.latent_phase_mean > 0.0);
    }

    #[test]
    fn missing_channel_is_omitted_without_being_filled_with_zero() {
        let mut history = history(Vec::new());
        history
            .observations
            .push(observation("s1", "session-a", 0, &[("joint-angle", 0.75)]));
        let forecast = forecast_next(
            &model("m", 0.5, 1.0),
            &history,
            "session-a",
            1,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        assert_eq!(forecast.conditioned_observation_count, 1);
        assert_eq!(forecast.channel_ids.len(), 2);
        assert!(
            forecast
                .predictive_means
                .iter()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn animal_effect_posterior_is_retained_while_phase_resets_between_sessions() {
        let model = model("m", 0.8, 1.0);
        let history = history(vec![observation(
            "s1",
            "session-a",
            0,
            &[("joint-angle", 0.6), ("contact-force", 1.2)],
        )]);
        let new_session = forecast_next(
            &model,
            &history,
            "session-b",
            0,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        assert_eq!(new_session.latent_phase_mean, 0.0);
        let channel = new_session
            .channel_ids
            .iter()
            .position(|id| id == "joint-angle")
            .unwrap();
        assert!(new_session.predictive_means[channel] > 0.0);
    }

    #[test]
    fn conditional_multichannel_likelihood_updates_competing_mechanisms() {
        let mechanisms = vec![
            model("weak-phase", 0.0, 0.1),
            model("strong-phase", 0.0, 1.0),
        ];
        let mut history = history(Vec::new());
        let receipt = update_mechanism_posterior(
            &mut history,
            &mechanisms,
            &prior(&["weak-phase", "strong-phase"], &[0.5, 0.5]),
            observation(
                "sample-1",
                "session-a",
                0,
                &[("joint-angle", 1.0), ("contact-force", 2.0)],
            ),
            MultivariateConfig::default(),
        )
        .unwrap();
        assert!(receipt.posterior_after["strong-phase"] > receipt.posterior_after["weak-phase"]);
        assert_eq!(receipt.history_observation_count, 1);
        assert_eq!(receipt.forecasts_before_observation.len(), 2);
    }

    #[test]
    fn rejects_unknown_channels_and_unknown_actions() {
        let unknown_channel = forecast_next(
            &model("m", 0.5, 1.0),
            &history(vec![observation("s1", "session-a", 0, &[("unknown", 0.0)])]),
            "session-a",
            1,
            "baseline",
            MultivariateConfig::default(),
        );
        assert!(matches!(
            unknown_channel,
            Err(MultivariateError::UnknownChannel { .. })
        ));

        let unknown_action = forecast_next(
            &model("m", 0.5, 1.0),
            &history(Vec::new()),
            "session-a",
            0,
            "not-registered",
            MultivariateConfig::default(),
        );
        assert!(matches!(
            unknown_action,
            Err(MultivariateError::InvalidActionEffect { .. })
        ));
    }

    #[test]
    fn irregular_time_and_session_reopening_fail_closed() {
        let irregular = history(vec![observation(
            "s1",
            "session-a",
            0,
            &[("joint-angle", 0.0)],
        )]);
        assert!(matches!(
            forecast_next(
                &model("m", 0.5, 1.0),
                &irregular,
                "session-a",
                2,
                "baseline",
                MultivariateConfig::default()
            ),
            Err(MultivariateError::NonContiguousTime { .. })
        ));

        let reopened = history(vec![
            observation("s1", "session-a", 0, &[("joint-angle", 0.0)]),
            observation("s2", "session-b", 0, &[("joint-angle", 0.0)]),
        ]);
        assert_eq!(
            forecast_next(
                &model("m", 0.5, 1.0),
                &reopened,
                "session-a",
                1,
                "baseline",
                MultivariateConfig::default()
            ),
            Err(MultivariateError::SessionReopened("session-a".to_owned()))
        );
    }

    #[test]
    fn channel_input_order_does_not_change_named_forecasts_or_covariance() {
        let model_forward = model("m", 0.7, 1.0);
        let mut model_reversed = model_forward.clone();
        model_reversed.channels.reverse();
        let history = history(vec![
            observation(
                "s1",
                "session-a",
                0,
                &[("joint-angle", 0.3), ("contact-force", 0.6)],
            ),
            observation(
                "s2",
                "session-a",
                1,
                &[("joint-angle", 0.4), ("contact-force", 0.8)],
            ),
        ]);
        let forward = forecast_next(
            &model_forward,
            &history,
            "session-a",
            2,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        let reversed = forecast_next(
            &model_reversed,
            &history,
            "session-a",
            2,
            "baseline",
            MultivariateConfig::default(),
        )
        .unwrap();
        assert_eq!(forward.channel_ids, reversed.channel_ids);
        for (left, right) in forward
            .predictive_means
            .iter()
            .zip(&reversed.predictive_means)
        {
            assert!((left - right).abs() < 1.0e-10);
        }
        for (left_row, right_row) in forward
            .predictive_covariance
            .iter()
            .zip(&reversed.predictive_covariance)
        {
            for (left, right) in left_row.iter().zip(right_row) {
                assert!((left - right).abs() < 1.0e-10);
            }
        }
        let determinant = forward.predictive_covariance[0][0] * forward.predictive_covariance[1][1]
            - forward.predictive_covariance[0][1] * forward.predictive_covariance[1][0];
        assert!(determinant > 0.0);
    }

    #[test]
    fn failed_update_does_not_mutate_history() {
        let mut history = history(Vec::new());
        let mut invalid = observation("s1", "session-a", 0, &[("joint-angle", 0.0)]);
        invalid.assigned_action_id = "stim".to_owned();
        let result = update_mechanism_posterior(
            &mut history,
            &[model("m", 0.5, 1.0)],
            &prior(&["m"], &[1.0]),
            invalid,
            MultivariateConfig::default(),
        );
        assert_eq!(result, Err(MultivariateError::AssignmentDeliveryMismatch));
        assert!(history.observations.is_empty());
    }
}
