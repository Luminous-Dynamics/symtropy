//! Compact nonlinear raw-state baseline for one-step residual prediction.
//!
//! This model is deliberately independent of HDC/LTC. It uses the exact same fixed,
//! causal state/action/time feature transform as LinearResidualRidge, then trains a
//! small one-hidden-layer tanh MLP with deterministic initialization and full-batch Adam.
//! It is a stronger comparator than linear ridge without being an end-to-end HDC/LTC
//! model. No test-set state, hidden disturbance label, or future value enters training.

use crate::linear_residual_ridge::{
    features as causal_features, ResidualTrainingSample, INPUT_FEATURES,
};
use symthaea_multirotor::{FlightState, QuadrotorCommand};

const HIDDEN_UNITS: usize = 16;
const OUTPUTS: usize = 3;
const TARGET_SCALE_M_S2: f64 = 10.0;
const TRAINING_EPOCHS: usize = 160;
const LEARNING_RATE: f64 = 0.003;
const L2_PENALTY: f64 = 1.0e-5;
const BETA1: f64 = 0.9;
const BETA2: f64 = 0.999;
const ADAM_EPSILON: f64 = 1.0e-8;
const MAX_GRADIENT_NORM: f64 = 5.0;
pub const INITIALIZATION_SEED: u64 = 0x434f_4d50_4d4c_5001;

const W1_START: usize = 0;
const B1_START: usize = W1_START + HIDDEN_UNITS * INPUT_FEATURES;
const W2_START: usize = B1_START + HIDDEN_UNITS;
const B2_START: usize = W2_START + OUTPUTS * HIDDEN_UNITS;
const PARAMETER_COUNT: usize = B2_START + OUTPUTS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactMlpError {
    EmptyTrainingSet,
    InvalidSample,
    NonFiniteParameters,
}

/// One-hidden-layer tanh MLP trained on fixed-scale causal raw-state features.
/// Model topology and optimizer are versioned constants so the run receipt is auditable.
#[derive(Debug, Clone)]
pub struct CompactMlpResidual {
    parameters: Vec<f64>,
    training_sample_count: usize,
    final_training_rmse_m_s2: f64,
}

impl CompactMlpResidual {
    pub fn fit(samples: &[ResidualTrainingSample]) -> Result<Self, CompactMlpError> {
        if samples.is_empty() {
            return Err(CompactMlpError::EmptyTrainingSet);
        }

        let mut inputs = Vec::with_capacity(samples.len());
        let mut targets = Vec::with_capacity(samples.len());
        for sample in samples {
            let expanded = causal_features(&sample.state_t, &sample.applied_action, sample.dt_s)
                .ok_or(CompactMlpError::InvalidSample)?;
            if !sample
                .target_residual_acceleration_world_m_s2
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(CompactMlpError::InvalidSample);
            }
            inputs.push(std::array::from_fn(|index| expanded[index + 1]));
            targets.push(sample.target_residual_acceleration_world_m_s2.map(
                |value| value / TARGET_SCALE_M_S2,
            ));
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
                    for input_index in 0..INPUT_FEATURES {
                        gradient[W1_START + hidden_index * INPUT_FEATURES + input_index] +=
                            hidden_delta * input[input_index];
                    }
                }
            }

            // L2 only on weights, never biases.
            for weight in W1_START..B1_START {
                gradient[weight] += L2_PENALTY * parameters[weight];
            }
            for weight in W2_START..B2_START {
                gradient[weight] += L2_PENALTY * parameters[weight];
            }

            let norm = gradient.iter().map(|value| value * value).sum::<f64>().sqrt();
            if !norm.is_finite() {
                return Err(CompactMlpError::NonFiniteParameters);
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
                return Err(CompactMlpError::NonFiniteParameters);
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
            return Err(CompactMlpError::NonFiniteParameters);
        }

        Ok(Self {
            parameters,
            training_sample_count: samples.len(),
            final_training_rmse_m_s2,
        })
    }

    pub fn predict(
        &self,
        state_t: &FlightState,
        applied_action: &QuadrotorCommand,
        dt_s: f64,
    ) -> Result<[f64; OUTPUTS], CompactMlpError> {
        let expanded =
            causal_features(state_t, applied_action, dt_s).ok_or(CompactMlpError::InvalidSample)?;
        let input = std::array::from_fn(|index| expanded[index + 1]);
        let (_, normalized) = forward(&input, &self.parameters);
        let result = normalized.map(|value| value * TARGET_SCALE_M_S2);
        if result.iter().all(|value| value.is_finite()) {
            Ok(result)
        } else {
            Err(CompactMlpError::NonFiniteParameters)
        }
    }

    pub fn training_sample_count(&self) -> usize {
        self.training_sample_count
    }

    pub fn final_training_rmse_m_s2(&self) -> f64 {
        self.final_training_rmse_m_s2
    }

    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
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

fn forward(input: &[f64; INPUT_FEATURES], parameters: &[f64]) -> ([f64; HIDDEN_UNITS], [f64; OUTPUTS]) {
    let mut hidden = [0.0; HIDDEN_UNITS];
    for hidden_index in 0..HIDDEN_UNITS {
        let mut activation = parameters[B1_START + hidden_index];
        for input_index in 0..INPUT_FEATURES {
            activation += parameters[W1_START + hidden_index * INPUT_FEATURES + input_index]
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
    let hidden_scale = (6.0 / (INPUT_FEATURES + HIDDEN_UNITS) as f64).sqrt();
    let output_scale = (6.0 / (HIDDEN_UNITS + OUTPUTS) as f64).sqrt();

    for index in W1_START..B1_START {
        parameters[index] = signed_unit(index) * hidden_scale;
    }
    for index in W2_START..B2_START {
        parameters[index] = signed_unit(index) * output_scale;
    }
    // Biases are exactly zero at initialization.
    parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(index: usize) -> ResidualTrainingSample {
        let i = index as f64;
        let state_t = FlightState {
            timestamp: i * 0.002,
            position: [
                0.12 * (i * 0.17).sin(),
                0.10 * (i * 0.11).cos(),
                0.1 + 0.08 * (i * 0.09).sin(),
            ],
            quaternion: [1.0, 0.0, 0.0, 0.0],
            linear_velocity: [
                2.0 * (i * 0.21).sin(),
                1.7 * (i * 0.14).cos(),
                1.2 * (i * 0.08).sin(),
            ],
            angular_velocity: [
                3.0 * (i * 0.10).sin(),
                2.5 * (i * 0.16).cos(),
                1.8 * (i * 0.23).sin(),
            ],
        };
        let action = QuadrotorCommand::from_raw([
            0.25 + 0.10 * (i * 0.12).sin() as f32,
            0.003 * (i * 0.15).sin() as f32,
            0.003 * (i * 0.18).cos() as f32,
            0.001 * (i * 0.24).sin() as f32,
        ]);
        let dt_s = 0.0015 + (index % 11) as f64 * 0.0001;
        let expanded = causal_features(&state_t, &action, dt_s).expect("valid test features");
        let x1 = expanded[1];
        let x2 = expanded[2];
        let x8 = expanded[8];
        ResidualTrainingSample {
            state_t,
            applied_action: action,
            dt_s,
            target_residual_acceleration_world_m_s2: [
                0.4 * (x1 * x2).tanh() + 0.1 * x8.sin(),
                -0.2 * (x2 * x8).tanh(),
                0.15 * x1.sin() + 0.1 * x8.cos(),
            ],
        }
    }

    #[test]
    fn fixed_seed_training_is_deterministic_and_compact() {
        let train = (0..48).map(sample).collect::<Vec<_>>();
        let first = CompactMlpResidual::fit(&train).expect("finite training samples");
        let second = CompactMlpResidual::fit(&train).expect("same fixed-seed fit");
        assert_eq!(first.parameter_count(), 355);
        assert_eq!(first.parameter_count(), second.parameter_count());
        assert_eq!(
            first.final_training_rmse_m_s2().to_bits(),
            second.final_training_rmse_m_s2().to_bits()
        );
        assert_eq!(first.training_sample_count(), train.len());
        assert!(first.final_training_rmse_m_s2().is_finite());
        assert_eq!(first.training_epochs(), TRAINING_EPOCHS);
    }

    #[test]
    fn rejects_empty_or_non_finite_training_data() {
        assert_eq!(
            CompactMlpResidual::fit(&[]).err(),
            Some(CompactMlpError::EmptyTrainingSet)
        );
        let mut invalid = sample(4);
        invalid.target_residual_acceleration_world_m_s2[1] = f64::NAN;
        assert_eq!(
            CompactMlpResidual::fit(&[invalid]).err(),
            Some(CompactMlpError::InvalidSample)
        );
    }

    #[test]
    fn predict_rejects_invalid_state_and_returns_finite_residual() {
        let train = (0..32).map(sample).collect::<Vec<_>>();
        let model = CompactMlpResidual::fit(&train).expect("valid training data");
        let probe = sample(33);
        let prediction = model
            .predict(&probe.state_t, &probe.applied_action, probe.dt_s)
            .expect("finite input should produce finite output");
        assert!(prediction.iter().all(|value| value.is_finite()));

        let mut invalid_state = probe.state_t;
        invalid_state.quaternion = [0.0; 4];
        assert_eq!(
            model
                .predict(&invalid_state, &probe.applied_action, probe.dt_s)
                .err(),
            Some(CompactMlpError::InvalidSample)
        );
    }
}
