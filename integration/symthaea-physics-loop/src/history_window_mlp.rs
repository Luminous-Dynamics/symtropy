//! Explicit causal delay embedding for a matched temporal baseline.
//!
//! Current state/action/dt plus the four preceding state/action/dt feature vectors form a
//! fixed causal history window. Each prior lag has an availability mask, so episode-start
//! padding is distinguishable from a real all-zero observation. This is a raw-history
//! baseline—not an HDC/LTC representation—and it helps separate memory value from
//! representation value in the temporal reservoir experiment.

use crate::linear_residual_ridge::{
    features as instantaneous_features, INPUT_FEATURES,
};
use symthaea_multirotor::{FlightState, QuadrotorCommand};

pub const HISTORY_DEPTH: usize = 5;
pub const HISTORY_LAGS: usize = HISTORY_DEPTH - 1;
pub const HISTORY_INPUT_DIM: usize = INPUT_FEATURES * HISTORY_DEPTH + HISTORY_LAGS;

const HIDDEN_UNITS: usize = 16;
const OUTPUTS: usize = 3;
const TARGET_SCALE_M_S2: f64 = 10.0;
const TRAINING_EPOCHS: usize = 120;
const LEARNING_RATE: f64 = 0.003;
const L2_PENALTY: f64 = 1.0e-5;
const BETA1: f64 = 0.9;
const BETA2: f64 = 0.999;
const ADAM_EPSILON: f64 = 1.0e-8;
const MAX_GRADIENT_NORM: f64 = 5.0;
pub const INITIALIZATION_SEED: u64 = 0x4849_5354_4d4c_5001;

const W1_START: usize = 0;
const B1_START: usize = W1_START + HIDDEN_UNITS * HISTORY_INPUT_DIM;
const W2_START: usize = B1_START + HIDDEN_UNITS;
const B2_START: usize = W2_START + OUTPUTS * HIDDEN_UNITS;
const PARAMETER_COUNT: usize = B2_START + OUTPUTS;

/// A supervised example whose input was constructed only from the causal history
/// available before the target transition. Target is observed-minus-nominal acceleration.
#[derive(Debug, Clone)]
pub struct HistoryTrainingSample {
    pub features: [f64; HISTORY_INPUT_DIM],
    pub target_residual_acceleration_world_m_s2: [f64; OUTPUTS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryMlpError {
    EmptyTrainingSet,
    InvalidSample,
    InvalidHistory,
    NonFiniteParameters,
}

/// Transform one current transition and an optional newest-first causal history window.
/// Historical entries are the 18 non-intercept features from prior state/action/dt rows.
/// At most four previous transitions are accepted; absent lag blocks remain zero and have
/// an explicit zero mask. No target/future-state field is accepted by this interface.
pub fn history_features(
    state_t: &FlightState,
    applied_action: &QuadrotorCommand,
    dt_s: f64,
    previous_newest_first: &[[f64; INPUT_FEATURES]],
) -> Option<[f64; HISTORY_INPUT_DIM]> {
    if previous_newest_first.len() > HISTORY_LAGS {
        return None;
    }
    if previous_newest_first
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return None;
    }

    let current = instantaneous_features(state_t, applied_action, dt_s)?;
    let mut expanded = [0.0; HISTORY_INPUT_DIM];
    expanded[..INPUT_FEATURES]
        .copy_from_slice(&current[1..1 + INPUT_FEATURES]);

    // Each older lag contributes its fixed feature vector plus one availability bit.
    for lag in 0..HISTORY_LAGS {
        let start = INPUT_FEATURES + lag * (INPUT_FEATURES + 1);
        if let Some(previous) = previous_newest_first.get(lag) {
            expanded[start..start + INPUT_FEATURES].copy_from_slice(previous);
            expanded[start + INPUT_FEATURES] = 1.0;
        }
    }
    expanded.iter().all(|value| value.is_finite()).then_some(expanded)
}

fn forward(
    input: &[f64; HISTORY_INPUT_DIM],
    parameters: &[f64],
) -> ([f64; HIDDEN_UNITS], [f64; OUTPUTS]) {
    let mut hidden = [0.0; HIDDEN_UNITS];
    for hidden_index in 0..HIDDEN_UNITS {
        let mut activation = parameters[B1_START + hidden_index];
        for input_index in 0..HISTORY_INPUT_DIM {
            activation +=
                parameters[W1_START + hidden_index * HISTORY_INPUT_DIM + input_index]
                    * input[input_index];
        }
        hidden[hidden_index] = activation.tanh();
    }

    let mut output = [0.0; OUTPUTS];
    for output_index in 0..OUTPUTS {
        let mut activation = parameters[B2_START + output_index];
        for hidden_index in 0..HIDDEN_UNITS {
            activation += parameters[W2_START + output_index * HIDDEN_UNITS + hidden_index]
                * hidden[hidden_index];
        }
        output[output_index] = activation;
    }
    (hidden, output)
}

fn mix_u64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn signed_unit(index: usize) -> f64 {
    let mantissa = mix_u64(INITIALIZATION_SEED ^ index as u64) >> 11;
    let unit = mantissa as f64 / ((1_u64 << 53) as f64);
    2.0 * unit - 1.0
}

fn initialized_parameters() -> Vec<f64> {
    let mut parameters = vec![0.0; PARAMETER_COUNT];
    let hidden_scale = (6.0 / (HISTORY_INPUT_DIM + HIDDEN_UNITS) as f64).sqrt();
    let output_scale = (6.0 / (HIDDEN_UNITS + OUTPUTS) as f64).sqrt();
    for index in W1_START..B1_START {
        parameters[index] = signed_unit(index) * hidden_scale;
    }
    for index in W2_START..B2_START {
        parameters[index] = signed_unit(index) * output_scale;
    }
    parameters
}

/// Compact nonlinear model with explicit finite-memory delay embedding.
/// Trainable parameter count and optimizer settings are exposed in the evidence receipt.
#[derive(Debug, Clone)]
pub struct HistoryWindowMlpResidual {
    parameters: Vec<f64>,
    training_sample_count: usize,
    final_training_rmse_m_s2: f64,
}

impl HistoryWindowMlpResidual {
    pub fn fit(samples: &[HistoryTrainingSample]) -> Result<Self, HistoryMlpError> {
        if samples.is_empty() {
            return Err(HistoryMlpError::EmptyTrainingSet);
        }

        let mut inputs = Vec::with_capacity(samples.len());
        let mut targets = Vec::with_capacity(samples.len());
        for sample in samples {
            if !sample.features.iter().all(|value| value.is_finite())
                || !sample
                    .target_residual_acceleration_world_m_s2
                    .iter()
                    .all(|value| value.is_finite())
            {
                return Err(HistoryMlpError::InvalidSample);
            }
            inputs.push(sample.features);
            targets.push(
                sample
                    .target_residual_acceleration_world_m_s2
                    .map(|value| value / TARGET_SCALE_M_S2),
            );
        }

        let mut parameters = initialized_parameters();
        let mut first_moment = vec![0.0; PARAMETER_COUNT];
        let mut second_moment = vec![0.0; PARAMETER_COUNT];

        for epoch in 1..=TRAINING_EPOCHS {
            let mut gradient = vec![0.0; PARAMETER_COUNT];
            for (input, target) in inputs.iter().zip(targets.iter()) {
                let (hidden, predicted) = forward(input, &parameters);
                let sample_scale = 1.0 / (inputs.len() * OUTPUTS) as f64;
                let mut output_delta = [0.0; OUTPUTS];

                for output in 0..OUTPUTS {
                    output_delta[output] = (predicted[output] - target[output]) * sample_scale;
                    gradient[B2_START + output] += output_delta[output];
                    for hidden_index in 0..HIDDEN_UNITS {
                        gradient[W2_START + output * HIDDEN_UNITS + hidden_index] +=
                            output_delta[output] * hidden[hidden_index];
                    }
                }

                for hidden_index in 0..HIDDEN_UNITS {
                    let mut hidden_delta = 0.0;
                    for output in 0..OUTPUTS {
                        hidden_delta += output_delta[output]
                            * parameters[W2_START + output * HIDDEN_UNITS + hidden_index];
                    }
                    hidden_delta *= 1.0 - hidden[hidden_index] * hidden[hidden_index];
                    gradient[B1_START + hidden_index] += hidden_delta;
                    for input_index in 0..HISTORY_INPUT_DIM {
                        gradient[W1_START + hidden_index * HISTORY_INPUT_DIM + input_index] +=
                            hidden_delta * input[input_index];
                    }
                }
            }

            for weight in W1_START..B1_START {
                gradient[weight] += L2_PENALTY * parameters[weight];
            }
            for weight in W2_START..B2_START {
                gradient[weight] += L2_PENALTY * parameters[weight];
            }

            let norm = gradient.iter().map(|value| value * value).sum::<f64>().sqrt();
            if !norm.is_finite() {
                return Err(HistoryMlpError::NonFiniteParameters);
            }
            if norm > MAX_GRADIENT_NORM {
                let scale = MAX_GRADIENT_NORM / norm;
                for value in &mut gradient {
                    *value *= scale;
                }
            }

            let epoch_f = epoch as f64;
            let correction1 = 1.0 - BETA1.powf(epoch_f);
            let correction2 = 1.0 - BETA2.powf(epoch_f);
            for index in 0..PARAMETER_COUNT {
                first_moment[index] =
                    BETA1 * first_moment[index] + (1.0 - BETA1) * gradient[index];
                second_moment[index] = BETA2 * second_moment[index]
                    + (1.0 - BETA2) * gradient[index] * gradient[index];
                let mean = first_moment[index] / correction1;
                let variance = second_moment[index] / correction2;
                parameters[index] -=
                    LEARNING_RATE * mean / (variance.sqrt() + ADAM_EPSILON);
            }
            if !parameters.iter().all(|value| value.is_finite()) {
                return Err(HistoryMlpError::NonFiniteParameters);
            }
        }

        let mut squared_error = 0.0;
        for (input, target) in inputs.iter().zip(targets.iter()) {
            let (_, predicted) = forward(input, &parameters);
            for output in 0..OUTPUTS {
                let error = (predicted[output] - target[output]) * TARGET_SCALE_M_S2;
                squared_error += error * error;
            }
        }
        let final_training_rmse_m_s2 =
            (squared_error / (samples.len() * OUTPUTS) as f64).sqrt();
        if !final_training_rmse_m_s2.is_finite() {
            return Err(HistoryMlpError::NonFiniteParameters);
        }

        Ok(Self {
            parameters,
            training_sample_count: samples.len(),
            final_training_rmse_m_s2,
        })
    }

    pub fn predict_features(
        &self,
        features: &[f64; HISTORY_INPUT_DIM],
    ) -> Result<[f64; OUTPUTS], HistoryMlpError> {
        if !features.iter().all(|value| value.is_finite()) {
            return Err(HistoryMlpError::InvalidSample);
        }
        let (_, normalized) = forward(features, &self.parameters);
        let predicted = normalized.map(|value| value * TARGET_SCALE_M_S2);
        if predicted.iter().all(|value| value.is_finite()) {
            Ok(predicted)
        } else {
            Err(HistoryMlpError::NonFiniteParameters)
        }
    }

    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
    }

    pub fn training_sample_count(&self) -> usize {
        self.training_sample_count
    }

    pub fn final_training_rmse_m_s2(&self) -> f64 {
        self.final_training_rmse_m_s2
    }

    pub fn training_epochs(&self) -> usize {
        TRAINING_EPOCHS
    }

    pub fn learning_rate(&self) -> f64 {
        LEARNING_RATE
    }

    pub fn hidden_units(&self) -> usize {
        HIDDEN_UNITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(index: usize) -> HistoryTrainingSample {
        let mut features = [0.0; HISTORY_INPUT_DIM];
        let i = index as f64;
        for (j, value) in features.iter_mut().enumerate() {
            *value = (i * (j as f64 + 1.0) * 0.017).sin();
        }
        for lag in 0..HISTORY_LAGS {
            let mask = INPUT_FEATURES + lag * (INPUT_FEATURES + 1) + INPUT_FEATURES;
            features[mask] = 1.0;
        }
        HistoryTrainingSample {
            features,
            target_residual_acceleration_world_m_s2: [
                0.2 * features[0] + 0.1 * (features[3] * features[18]).tanh(),
                -0.1 * features[1] + 0.2 * features[4].sin(),
                0.15 * features[2] + 0.05 * features[20],
            ],
        }
    }

    #[test]
    fn history_mask_marks_available_lags_and_zero_pads_episode_start() {
        let state = FlightState::hover(0.1);
        let action = QuadrotorCommand::hover();
        let current = history_features(&state, &action, 0.002, &[]).expect("valid current row");
        assert!(current[INPUT_FEATURES..].iter().all(|value| *value == 0.0));

        let previous = [0.25; INPUT_FEATURES];
        let with_history =
            history_features(&state, &action, 0.002, &[previous]).expect("valid prior row");
        let mask = INPUT_FEATURES + INPUT_FEATURES;
        assert_eq!(with_history[mask], 1.0);
        assert_eq!(with_history[INPUT_FEATURES], 0.25);
        assert!(with_history[mask + 1..].iter().all(|value| *value == 0.0));
    }

    #[test]
    fn history_features_reject_too_many_or_non_finite_lags() {
        let state = FlightState::hover(0.1);
        let action = QuadrotorCommand::hover();
        let too_many = vec![[0.0; INPUT_FEATURES]; HISTORY_DEPTH];
        assert!(history_features(&state, &action, 0.002, &too_many).is_none());

        let mut invalid = [0.0; INPUT_FEATURES];
        invalid[2] = f64::NAN;
        assert!(history_features(&state, &action, 0.002, &[invalid]).is_none());
    }

    #[test]
    fn fixed_seed_fit_is_deterministic_and_has_declared_size() {
        let train = (0..40).map(sample).collect::<Vec<_>>();
        let first = HistoryWindowMlpResidual::fit(&train).expect("valid history training data");
        let second = HistoryWindowMlpResidual::fit(&train).expect("same fixed fit");
        assert_eq!(first.parameter_count(), 1_571);
        assert_eq!(first.parameter_count(), second.parameter_count());
        assert_eq!(
            first.final_training_rmse_m_s2().to_bits(),
            second.final_training_rmse_m_s2().to_bits()
        );
        assert_eq!(first.training_sample_count(), train.len());
    }

    #[test]
    fn history_mlp_rejects_empty_and_non_finite_training_samples() {
        assert_eq!(
            HistoryWindowMlpResidual::fit(&[]).err(),
            Some(HistoryMlpError::EmptyTrainingSet)
        );
        let mut bad = sample(3);
        bad.target_residual_acceleration_world_m_s2[2] = f64::INFINITY;
        assert_eq!(
            HistoryWindowMlpResidual::fit(&[bad]).err(),
            Some(HistoryMlpError::InvalidSample)
        );
    }
}
